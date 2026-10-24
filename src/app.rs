use crate::config::{
    AppConfig,
    MAX_RX_BUFFER_SIZE,
    MAX_SCRIPT_LOG_ENTRIES,
    MAX_SYSTEM_LOG_ENTRIES,
};
use crate::types::{
    LineEnding,
    PreconfigCommand,
    RxEvent,
    ToolTab,
    TxCmd,
    ViewMode,
};
use crate::ui;
use crate::worker::spawn_serial_worker;
use chrono::Local;
use crossbeam_channel::{
    Receiver,
    Sender,
    unbounded,
};
use eframe::egui;
use rhai::Engine;
use serialport::{
    DataBits,
    FlowControl,
    Parity,
    StopBits,
};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};
use std::sync::{
    Arc,
    Mutex,
};
use std::thread;
use std::time::{
    Duration,
    Instant,
};

pub struct SerialForgeApp {
    pub available_ports: Vec<String>,
    pub selected_port: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,

    pub baud_input: String,
    pub auto_reconnect: bool,
    pub reconnect_timer: Option<Instant>,

    pub rts: bool,
    pub dtr: bool,
    pub is_connected: bool,
    pub dark_mode: bool,

    pub cmd_tx: Sender<TxCmd>,
    pub evt_rx: Receiver<RxEvent>,
    pub script_rx_rx: Receiver<Vec<u8>>,

    pub rx_buffer: Vec<u8>,
    pub view_mode: ViewMode,
    pub cached_formatted_text: Option<String>,
    pub auto_scroll: bool,
    pub rx_bytes: usize,
    pub tx_bytes: usize,

    // Session Logging (Log to File)
    pub log_file: Option<File>,
    pub log_file_path: Option<PathBuf>,
    pub log_file_bytes: usize,

    // Tools Tab Selection
    pub active_tab: ToolTab,

    // Sending & History
    pub send_text: String,
    pub send_as_hex: bool,
    pub line_ending: LineEnding,
    pub cmd_history: Vec<String>,
    pub cmd_history_idx: Option<usize>,

    // Periodic / Repeat Transmit
    pub repeat_send_enabled: bool,
    pub repeat_send_interval_ms: u64,
    pub last_repeat_send: Option<Instant>,

    // Preconfigured Commands
    pub preconfig_commands: Vec<PreconfigCommand>,
    pub new_preconfig_name: String,
    pub new_preconfig_cmd: String,

    // File Transfer
    pub file_path: Option<PathBuf>,
    pub chunk_size: usize,
    pub chunk_delay_ms: u64,
    pub file_progress: Option<(usize, usize)>,

    // Scripting Engine
    pub script_text: String,
    pub script_running: Arc<AtomicBool>,
    pub script_log: Arc<Mutex<Vec<String>>>,

    // System Logs
    pub system_log: Vec<String>,
    pub status_msg: String,
}

impl SerialForgeApp {
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (cmd_tx, cmd_rx) = unbounded();
        let (evt_tx, evt_rx) = unbounded();
        let (script_rx_tx, script_rx_rx) = unbounded();

        let script_running = Arc::new(AtomicBool::new(false));

        spawn_serial_worker(
            cmd_rx,
            evt_tx,
            cc.egui_ctx.clone(),
            script_rx_tx,
            script_running.clone(),
        );

        let ports = serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.port_name)
            .collect::<Vec<_>>();

        // Restore persisted state if available
        let config: AppConfig = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();

        let selected_port =
            if !config.selected_port.is_empty() && ports.contains(&config.selected_port) {
                config.selected_port
            } else {
                ports.first().cloned().unwrap_or_default()
            };

        let data_bits = match config.data_bits {
            5 => DataBits::Five,
            6 => DataBits::Six,
            7 => DataBits::Seven,
            _ => DataBits::Eight,
        };

        let stop_bits = match config.stop_bits {
            2 => StopBits::Two,
            _ => StopBits::One,
        };

        let parity = match config.parity.as_str() {
            "Odd" => Parity::Odd,
            "Even" => Parity::Even,
            _ => Parity::None,
        };

        let flow_control = match config.flow_control.as_str() {
            "Software" => FlowControl::Software,
            "Hardware" => FlowControl::Hardware,
            _ => FlowControl::None,
        };

        let mut app = Self {
            available_ports: ports,
            selected_port,
            baud_rate: config.baud_rate,
            data_bits,
            stop_bits,
            parity,
            flow_control,
            baud_input: config.baud_rate.to_string(),
            auto_reconnect: config.auto_reconnect,
            reconnect_timer: None,
            rts: false,
            dtr: false,
            is_connected: false,
            dark_mode: config.dark_mode,
            cmd_tx,
            evt_rx,
            script_rx_rx,
            rx_buffer: Vec::new(),
            view_mode: config.view_mode,
            cached_formatted_text: None,
            auto_scroll: config.auto_scroll,
            rx_bytes: 0,
            tx_bytes: 0,
            log_file: None,
            log_file_path: None,
            log_file_bytes: 0,
            active_tab: config.active_tab,
            send_text: String::new(),
            send_as_hex: false,
            line_ending: config.line_ending,
            cmd_history: Vec::new(),
            cmd_history_idx: None,
            repeat_send_enabled: false,
            repeat_send_interval_ms: 1000,
            last_repeat_send: None,
            preconfig_commands: config.preconfig_commands,
            new_preconfig_name: String::new(),
            new_preconfig_cmd: String::new(),
            file_path: None,
            chunk_size: config.chunk_size,
            chunk_delay_ms: config.chunk_delay_ms,
            file_progress: None,
            script_text: r#"// serialforge Rhai Script Automation Example
let counter = 0;
while counter < 5 {
    print(`Executing Step ${counter + 1}...`);
    serial_write(`STATUS_QUERY_${counter}`);
    sleep_ms(300);
    let resp = serial_read_line(500);
    print(`Got response: ${resp}`);
    counter += 1;
}
print("Automation sequence completed.");
"#
            .into(),
            script_running,
            script_log: Arc::new(Mutex::new(Vec::new())),
            system_log: Vec::new(),
            status_msg: "Disconnected".to_string(),
        };

        app.log_system("Application initialized.");
        app
    }

    pub fn log_system(&mut self, msg: &str) {
        let time = Local::now().format("%H:%M:%S%.3f").to_string();
        self.system_log.push(format!("[{time}] {msg}"));
        if self.system_log.len() > MAX_SYSTEM_LOG_ENTRIES {
            let excess = self.system_log.len() - MAX_SYSTEM_LOG_ENTRIES;
            self.system_log.drain(0..excess);
        }
    }

    pub fn refresh_ports(&mut self) {
        if let Ok(ports) = serialport::available_ports() {
            self.available_ports = ports.into_iter().map(|p| p.port_name).collect();
            if !self.available_ports.contains(&self.selected_port)
                && !self.available_ports.is_empty()
            {
                self.selected_port = self.available_ports[0].clone();
            }
            self.log_system("Refreshed serial ports list.");
        }
    }

    fn handle_events(&mut self) {
        while let Ok(evt) = self.evt_rx.try_recv() {
            match evt {
                RxEvent::Connected(port) => {
                    self.is_connected = true;
                    self.reconnect_timer = None;
                    self.status_msg = format!("Connected to {port}");
                    self.log_system(&format!("Successfully connected to {port}"));
                }
                RxEvent::Disconnected => {
                    self.is_connected = false;
                    self.status_msg = "Disconnected".to_string();
                    self.log_system("Port disconnected.");
                    if self.auto_reconnect && !self.selected_port.is_empty() {
                        self.reconnect_timer = Some(Instant::now() + Duration::from_millis(1500));
                        self.log_system("Auto-reconnect scheduled in 1.5s...");
                    }
                }
                RxEvent::DataReceived(bytes) => {
                    self.rx_bytes += bytes.len();
                    if let Some(ref mut f) = self.log_file
                        && f.write_all(&bytes).is_ok()
                    {
                        self.log_file_bytes += bytes.len();
                        let _ = f.flush();
                    }
                    self.rx_buffer.extend(bytes);
                    if self.rx_buffer.len() > MAX_RX_BUFFER_SIZE {
                        let excess = self.rx_buffer.len() - MAX_RX_BUFFER_SIZE;
                        self.rx_buffer.drain(0..excess);
                    }
                    self.cached_formatted_text = None;
                }
                RxEvent::FileSendProgress { sent, total } => {
                    self.file_progress = Some((sent, total));
                    self.status_msg = format!("Sending file: {sent}/{total} bytes");
                }
                RxEvent::FileSendComplete => {
                    self.file_progress = None;
                    self.status_msg = "File transfer complete!".to_string();
                    self.log_system("Binary file transfer finished.");
                }
                RxEvent::Error(e) => {
                    self.file_progress = None;
                    self.status_msg = format!("Error: {e}");
                    self.log_system(&format!("ERROR: {e}"));
                    if self.auto_reconnect
                        && !self.is_connected
                        && !self.selected_port.is_empty()
                        && self.reconnect_timer.is_none()
                    {
                        self.reconnect_timer = Some(Instant::now() + Duration::from_millis(2000));
                    }
                }
            }
        }

        if let Some(deadline) = self.reconnect_timer
            && Instant::now() >= deadline
        {
            self.reconnect_timer = None;
            if !self.is_connected && !self.selected_port.is_empty() {
                // Check if port is present in available ports or refresh
                if let Ok(ports) = serialport::available_ports() {
                    self.available_ports = ports.into_iter().map(|p| p.port_name).collect();
                }
                if self.available_ports.contains(&self.selected_port) {
                    self.log_system(&format!("Auto-reconnecting to {}...", self.selected_port));
                    let _ = self.cmd_tx.send(TxCmd::Connect {
                        port_name: self.selected_port.clone(),
                        baud_rate: self.baud_rate,
                        data_bits: self.data_bits,
                        stop_bits: self.stop_bits,
                        parity: self.parity,
                        flow_control: self.flow_control,
                    });
                } else {
                    // Retry again in 1.5s
                    self.reconnect_timer = Some(Instant::now() + Duration::from_millis(1500));
                }
            }
        }
    }

    pub fn send_string_command(&mut self, text: String) {
        if text.is_empty() {
            return;
        }

        if self.cmd_history.last() != Some(&text) {
            self.cmd_history.push(text.clone());
        }
        self.cmd_history_idx = None;

        let payload = if self.send_as_hex {
            let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            match hex::decode(&cleaned) {
                Ok(bytes) => bytes,
                Err(e) => {
                    self.log_system(&format!("HEX decode error: {e}. Payload not sent."));
                    return;
                }
            }
        } else {
            let mut p = text.into_bytes();
            p.extend_from_slice(self.line_ending.as_bytes());
            p
        };

        self.tx_bytes += payload.len();
        let _ = self.cmd_tx.send(TxCmd::SendData(payload));
    }

    pub fn insert_script_text(&mut self, ctx: &egui::Context, editor_id: egui::Id, text: &str) {
        let byte_offset = |value: &str, char_index: usize| {
            value
                .char_indices()
                .nth(char_index)
                .map_or(value.len(), |(byte_index, _)| byte_index)
        };

        if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id)
            && let Some(range) = state.cursor.char_range()
        {
            let min_char = range.primary.index.0.min(range.secondary.index.0);
            let max_char = range.primary.index.0.max(range.secondary.index.0);
            let start = byte_offset(&self.script_text, min_char);
            let end = byte_offset(&self.script_text, max_char);
            self.script_text.replace_range(start..end, text);

            let new_char_pos = min_char + text.chars().count();
            state
                .cursor
                .set_char_range(Some(egui::text_selection::CCursorRange::one(
                    egui::text::CCursor::new(new_char_pos),
                )));
            state.store(ctx, editor_id);
        } else {
            self.script_text.push_str(text);
        }
    }

    pub fn run_script(&mut self, ctx: &egui::Context) {
        if self.script_running.load(Ordering::Relaxed) {
            return;
        }

        // Drain any stale messages from the script channel
        while self.script_rx_rx.try_recv().is_ok() {}

        let script = self.script_text.clone();
        let cmd_tx = self.cmd_tx.clone();
        let script_rx_rx = self.script_rx_rx.clone();
        let is_running = self.script_running.clone();
        let log = self.script_log.clone();
        let ctx_clone = ctx.clone();

        is_running.store(true, Ordering::Relaxed);
        log.lock().unwrap().clear();
        self.log_system("Started executing Rhai script...");

        thread::spawn(move || {
            let mut engine = Engine::new();

            let log_clone = log.clone();
            let ctx_print = ctx_clone.clone();
            engine.on_print(move |s| {
                let mut guard = log_clone.lock().unwrap();
                guard.push(s.to_string());
                if guard.len() > MAX_SCRIPT_LOG_ENTRIES {
                    let excess = guard.len() - MAX_SCRIPT_LOG_ENTRIES;
                    guard.drain(0..excess);
                }
                ctx_print.request_repaint();
            });

            let is_running_engine = is_running.clone();
            engine.on_progress(move |_| {
                if is_running_engine.load(Ordering::Relaxed) {
                    None
                } else {
                    Some(rhai::Dynamic::UNIT)
                }
            });

            let cmd_tx_send = cmd_tx.clone();
            engine.register_fn("serial_send", move |s: &str| {
                let _ = cmd_tx_send.send(TxCmd::SendData(s.as_bytes().to_vec()));
            });

            let cmd_tx_hex = cmd_tx.clone();
            engine.register_fn("serial_write_hex", move |hex_str: &str| {
                let cleaned: String = hex_str.chars().filter(|c| !c.is_whitespace()).collect();
                if let Ok(bytes) = hex::decode(&cleaned) {
                    let _ = cmd_tx_hex.send(TxCmd::SendData(bytes));
                }
            });

            // Write raw array of integers/bytes
            let cmd_tx_bytes = cmd_tx.clone();
            engine.register_fn("serial_write_bytes", move |bytes_arr: rhai::Array| {
                let mut data = Vec::with_capacity(bytes_arr.len());
                for item in bytes_arr {
                    if let Ok(val) = item.as_int() {
                        data.push(val as u8);
                    }
                }
                let _ = cmd_tx_bytes.send(TxCmd::SendData(data));
            });

            // File export / append report helper
            engine.register_fn(
                "export_report",
                move |filename: &str, content: &str| -> bool {
                    use std::fs::OpenOptions;
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(filename)
                        .and_then(|mut f| f.write_all(content.as_bytes()))
                        .is_ok()
                },
            );

            let rx = script_rx_rx.clone();
            let is_running_read = is_running.clone();
            engine.register_fn("serial_read_line", move |timeout_ms: i64| -> String {
                let timeout = Duration::from_millis(u64::try_from(timeout_ms.max(0)).unwrap_or(0));
                let start = Instant::now();
                let mut accumulated = String::new();
                loop {
                    if !is_running_read.load(Ordering::Relaxed) {
                        break;
                    }
                    if start.elapsed() > timeout {
                        break;
                    }
                    if let Ok(bytes) = rx.try_recv() {
                        let text = String::from_utf8_lossy(&bytes);
                        accumulated.push_str(&text);
                        if accumulated.contains('\r') || accumulated.contains('\n') {
                            break;
                        }
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                accumulated
            });

            let rx_wait = script_rx_rx.clone();
            let is_running_wait = is_running.clone();
            engine.register_fn(
                "serial_wait_for",
                move |pattern: &str, timeout_ms: i64| -> bool {
                    let timeout =
                        Duration::from_millis(u64::try_from(timeout_ms.max(0)).unwrap_or(0));
                    let start = Instant::now();
                    let mut accumulated = String::new();
                    while is_running_wait.load(Ordering::Relaxed) && start.elapsed() <= timeout {
                        if let Ok(bytes) = rx_wait.try_recv() {
                            let text = String::from_utf8_lossy(&bytes);
                            accumulated.push_str(&text);
                            if accumulated.contains(pattern) {
                                return true;
                            }
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                    false
                },
            );

            engine.register_fn("sleep_ms", move |ms: i64| {
                let duration = Duration::from_millis(u64::try_from(ms.max(0)).unwrap_or(0));
                thread::sleep(duration);
            });

            if let Err(e) = engine.run(&script) {
                if !is_running.load(Ordering::Relaxed) {
                    log.lock()
                        .unwrap()
                        .push("[INFO] Script execution stopped by user.".into());
                } else {
                    log.lock()
                        .unwrap()
                        .push(format!("[ERROR] Script error: {e}"));
                }
            }

            is_running.store(false, Ordering::Relaxed);
            ctx_clone.request_repaint();
        });
    }

    pub fn compute_formatted_text(&self) -> String {
        use std::fmt::Write;
        match self.view_mode {
            ViewMode::Ascii => String::from_utf8_lossy(&self.rx_buffer).to_string(),
            ViewMode::Hex => {
                let mut out = String::with_capacity(
                    self.rx_buffer.len().saturating_mul(3)
                        + self.rx_buffer.len().checked_div(16).unwrap_or(0),
                );
                for (i, chunk) in self.rx_buffer.chunks(16).enumerate() {
                    if i > 0 {
                        out.push('\n');
                    }
                    for (j, &b) in chunk.iter().enumerate() {
                        if j > 0 {
                            out.push(' ');
                        }
                        let _ = write!(out, "{b:02X}");
                    }
                }
                out
            }
            ViewMode::Mixed => {
                let mut out = String::with_capacity(self.rx_buffer.len().saturating_mul(5));
                for (i, chunk) in self.rx_buffer.chunks(16).enumerate() {
                    if i > 0 {
                        out.push('\n');
                    }
                    let mut hex_part = String::with_capacity(48);
                    for (j, &b) in chunk.iter().enumerate() {
                        if j > 0 {
                            hex_part.push(' ');
                        }
                        let _ = write!(hex_part, "{b:02X}");
                    }
                    let _ = write!(out, "{hex_part:<48} | ");
                    for &b in chunk {
                        if b.is_ascii_graphic() || b == b' ' {
                            out.push(b as char);
                        } else {
                            out.push('.');
                        }
                    }
                }
                out
            }
        }
    }
}

impl eframe::App for SerialForgeApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let config = AppConfig {
            selected_port: self.selected_port.clone(),
            baud_rate: self.baud_rate,
            data_bits: match self.data_bits {
                DataBits::Five => 5,
                DataBits::Six => 6,
                DataBits::Seven => 7,
                DataBits::Eight => 8,
            },
            stop_bits: match self.stop_bits {
                StopBits::Two => 2,
                _ => 1,
            },
            parity: format!("{}", self.parity),
            flow_control: format!("{}", self.flow_control),
            auto_reconnect: self.auto_reconnect,
            dark_mode: self.dark_mode,
            view_mode: self.view_mode,
            auto_scroll: self.auto_scroll,
            active_tab: self.active_tab,
            line_ending: self.line_ending,
            preconfig_commands: self.preconfig_commands.clone(),
            chunk_size: self.chunk_size,
            chunk_delay_ms: self.chunk_delay_ms,
        };
        eframe::set_value(storage, eframe::APP_KEY, &config);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        self.handle_events();

        // Handle periodic transmit if enabled and connected
        if self.repeat_send_enabled && self.is_connected && !self.send_text.is_empty() {
            let interval = Duration::from_millis(self.repeat_send_interval_ms.max(10));
            let now = Instant::now();
            let should_send = match self.last_repeat_send {
                None => true,
                Some(last) => now.duration_since(last) >= interval,
            };
            if should_send {
                self.last_repeat_send = Some(now);
                let text = self.send_text.clone();
                self.send_string_command(text);
            }
            ctx.request_repaint_after(interval);
        }

        // 1. TOP PANEL: Hardware Setup & Theme Switcher
        egui::Panel::top("top_panel").show(ui, |ui| {
            ui::toolbar::render_toolbar(self, ui);
        });

        // 2. BOTTOM PANEL: System Status Summary
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui::status_bar::render_status_bar(self, ui);
        });

        // 3. BOTTOM TABBED TOOLBAR: Send & Presets, File Transfer, Scripting, Log Console
        let tools_panel = egui::Panel::bottom("serialforge_tools_panel")
            .default_size(500.0)
            .min_size(280.0)
            .max_size(680.0)
            .resizable(true);
        tools_panel.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.active_tab,
                    ToolTab::TransmitPresets,
                    "💬 Send & Presets",
                );
                ui.selectable_value(&mut self.active_tab, ToolTab::BinaryFile, "📁 Binary File");
                ui.selectable_value(
                    &mut self.active_tab,
                    ToolTab::Scripting,
                    "📜 Rhai Scripting",
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    ToolTab::SystemLog,
                    "📋 System Console Log",
                );
            });

            ui.separator();

            match self.active_tab {
                ToolTab::TransmitPresets => {
                    ui::tabs::presets::render_presets_tab(self, ui);
                }
                ToolTab::BinaryFile => {
                    ui::tabs::binary_file::render_binary_file_tab(self, ui);
                }
                ToolTab::Scripting => {
                    ui::tabs::scripting::render_scripting_tab(self, ui);
                }
                ToolTab::SystemLog => {
                    ui::tabs::system_log::render_system_log_tab(self, ui);
                }
            }
        });

        // 4. CENTRAL PANEL: Primary Serial Terminal Display
        egui::CentralPanel::default().show(ui, |ui| {
            ui::terminal::render_terminal(self, ui);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> SerialForgeApp {
        let (cmd_tx, _cmd_rx) = unbounded();
        let (_evt_tx, evt_rx) = unbounded();
        let (_script_rx_tx, script_rx_rx) = unbounded();

        SerialForgeApp {
            available_ports: vec!["/dev/ttyUSB0".into()],
            selected_port: "/dev/ttyUSB0".into(),
            baud_rate: 115_200,
            data_bits: DataBits::Eight,
            stop_bits: StopBits::One,
            parity: Parity::None,
            flow_control: FlowControl::None,
            baud_input: "115200".into(),
            auto_reconnect: false,
            reconnect_timer: None,
            rts: false,
            dtr: false,
            is_connected: true,
            dark_mode: false,
            cmd_tx,
            evt_rx,
            script_rx_rx,
            rx_buffer: Vec::new(),
            view_mode: ViewMode::Ascii,
            cached_formatted_text: None,
            auto_scroll: true,
            rx_bytes: 0,
            tx_bytes: 0,
            log_file: None,
            log_file_path: None,
            log_file_bytes: 0,
            active_tab: ToolTab::TransmitPresets,
            send_text: String::new(),
            send_as_hex: false,
            line_ending: LineEnding::Crlf,
            cmd_history: Vec::new(),
            cmd_history_idx: None,
            repeat_send_enabled: false,
            repeat_send_interval_ms: 1000,
            last_repeat_send: None,
            preconfig_commands: Vec::new(),
            new_preconfig_name: String::new(),
            new_preconfig_cmd: String::new(),
            file_path: None,
            chunk_size: 256,
            chunk_delay_ms: 10,
            file_progress: None,
            script_text: String::new(),
            script_running: Arc::new(AtomicBool::new(false)),
            script_log: Arc::new(Mutex::new(Vec::new())),
            system_log: Vec::new(),
            status_msg: "Connected".into(),
        }
    }

    #[test]
    fn test_compute_formatted_text_ascii() {
        let mut app = create_test_app();
        app.rx_buffer = b"Hello, SerialForge!".to_vec();
        app.view_mode = ViewMode::Ascii;
        assert_eq!(app.compute_formatted_text(), "Hello, SerialForge!");
    }

    #[test]
    fn test_compute_formatted_text_hex() {
        let mut app = create_test_app();
        app.rx_buffer = vec![0xDE, 0xAD, 0xBE, 0xEF];
        app.view_mode = ViewMode::Hex;
        assert_eq!(app.compute_formatted_text(), "DE AD BE EF");
    }

    #[test]
    fn test_compute_formatted_text_mixed() {
        let mut app = create_test_app();
        app.rx_buffer = vec![0x41, 0x42, 0x00, 0x43];
        app.view_mode = ViewMode::Mixed;
        let formatted = app.compute_formatted_text();
        assert!(formatted.contains("41 42 00 43"));
        assert!(formatted.contains("AB.C"));
    }

    #[test]
    fn test_command_history_deduplication() {
        let mut app = create_test_app();
        app.send_string_command("AT".into());
        app.send_string_command("AT".into());
        assert_eq!(app.cmd_history, vec!["AT".to_string()]);
        app.send_string_command("ATI".into());
        assert_eq!(app.cmd_history, vec!["AT".to_string(), "ATI".to_string()]);
    }

    #[test]
    fn test_send_as_hex() {
        let mut app = create_test_app();
        app.send_as_hex = true;
        app.send_string_command("01 03 00 00 00 02 C4 0B".into());
        assert_eq!(app.tx_bytes, 8);
    }

    #[test]
    fn test_system_log_capping() {
        let mut app = create_test_app();
        for i in 0..MAX_SYSTEM_LOG_ENTRIES + 100 {
            app.log_system(&format!("log {i}"));
        }
        assert_eq!(app.system_log.len(), MAX_SYSTEM_LOG_ENTRIES);
    }
}
