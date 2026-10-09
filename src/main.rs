use chrono::Local;
use crossbeam_channel::{
    Receiver,
    Sender,
    unbounded,
};
use eframe::egui;
use rfd::FileDialog;
use rhai::Engine;
use serialport::{
    DataBits,
    FlowControl,
    Parity,
    SerialPort,
    StopBits,
};
use std::fs::File;
use std::io::{
    Read,
    Write,
};
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

// ==========================================
// 1. Thread Communication & Data Types
// ==========================================

#[derive(Debug, Clone)]
pub enum TxCmd {
    Connect {
        port_name: String,
        baud_rate: u32,
        data_bits: DataBits,
        stop_bits: StopBits,
        parity: Parity,
        flow_control: FlowControl,
    },
    Disconnect,
    SendData(Vec<u8>),
    SendFile {
        path: PathBuf,
        chunk_size: usize,
        delay_ms: u64,
    },
    SetRts(bool),
    SetDtr(bool),
}

#[derive(Debug, Clone)]
pub enum RxEvent {
    Connected(String),
    Disconnected,
    DataReceived(Vec<u8>),
    FileSendProgress { sent: usize, total: usize },
    FileSendComplete,
    Error(String),
}

#[derive(PartialEq, Clone, Copy)]
pub enum ViewMode {
    Ascii,
    Hex,
    Mixed,
}

#[derive(PartialEq, Clone, Copy)]
pub enum LineEnding {
    None,
    Null,
    CR,
    LF,
    CRLF,
    Etx,
}

impl LineEnding {
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            LineEnding::None => b"",
            LineEnding::Null => b"\x00",
            LineEnding::CR => b"\r",
            LineEnding::LF => b"\n",
            LineEnding::CRLF => b"\r\n",
            LineEnding::Etx => b"\x03",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreconfigCommand {
    pub name: String,
    pub cmd: String,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ToolTab {
    TransmitPresets,
    BinaryFile,
    Scripting,
    SystemLog,
}

// ==========================================
// 2. Syntax Colorizer for Rhai Scripting
// ==========================================

fn scrollable_script_field(
    ui: &mut egui::Ui,
    id: &str,
    editor: egui::TextEdit<'_>,
) -> egui::containers::scroll_area::ScrollAreaOutput<egui::Response> {
    egui::ScrollArea::both()
        .id_salt(id)
        .max_height(ui.available_height())
        .auto_shrink([false; 2])
        .show(ui, |ui| ui.add(editor))
}

fn highlight_rhai_code(ui: &egui::Ui, code: &str) -> egui::text::LayoutJob {
    use egui::text::LayoutJob;
    use egui::{
        Color32,
        TextFormat,
    };

    let is_dark = ui.visuals().dark_mode;
    let font_id = egui::FontId::monospace(13.0);

    let default_color = if is_dark {
        Color32::from_rgb(220, 220, 220)
    } else {
        Color32::from_rgb(30, 30, 30)
    };
    let keyword_color = if is_dark {
        Color32::from_rgb(86, 156, 214)
    } else {
        Color32::from_rgb(0, 0, 255)
    };
    let string_color = if is_dark {
        Color32::from_rgb(214, 157, 133)
    } else {
        Color32::from_rgb(163, 21, 21)
    };
    let comment_color = if is_dark {
        Color32::from_rgb(87, 166, 74)
    } else {
        Color32::from_rgb(0, 128, 0)
    };
    let fn_color = if is_dark {
        Color32::from_rgb(220, 220, 170)
    } else {
        Color32::from_rgb(121, 94, 38)
    };

    let mut job = LayoutJob::default();

    let keywords = [
        "let", "const", "if", "else", "while", "for", "in", "return", "fn", "break", "continue",
        "true", "false",
    ];
    let builtin_fns = ["print", "serial_send", "serial_read_line"];

    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Comments //
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
            let start = i;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            job.append(
                &text,
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color: comment_color,
                    ..Default::default()
                },
            );
            continue;
        }

        // String Literals "..."
        if chars[i] == '"' {
            let start = i;
            i += 1;
            while i < len && chars[i] != '"' && chars[i] != '\n' {
                if chars[i] == '\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            if i < len && chars[i] == '"' {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            job.append(
                &text,
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color: string_color,
                    ..Default::default()
                },
            );
            continue;
        }

        // Keywords / Identifiers
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let color = if keywords.contains(&word.as_str()) {
                keyword_color
            } else if builtin_fns.contains(&word.as_str()) {
                fn_color
            } else {
                default_color
            };
            job.append(
                &word,
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color,
                    ..Default::default()
                },
            );
            continue;
        }

        // Plain Text
        let ch = chars[i];
        i += 1;
        let mut s = String::new();
        s.push(ch);
        job.append(
            &s,
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: default_color,
                ..Default::default()
            },
        );
    }

    job
}

// ==========================================
// 3. Serial Worker Thread (I/O Loop)
// ==========================================

fn spawn_serial_worker(
    cmd_rx: Receiver<TxCmd>,
    evt_tx: Sender<RxEvent>,
    ctx: egui::Context,
    script_rx_tx: Sender<Vec<u8>>,
    script_running: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let mut port: Option<Box<dyn SerialPort>> = None;
        let mut read_buf = vec![0u8; 4096];

        loop {
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    TxCmd::Connect {
                        port_name,
                        baud_rate,
                        data_bits,
                        stop_bits,
                        parity,
                        flow_control,
                    } => {
                        let builder = serialport::new(&port_name, baud_rate)
                            .data_bits(data_bits)
                            .stop_bits(stop_bits)
                            .parity(parity)
                            .flow_control(flow_control)
                            .timeout(Duration::from_millis(10));

                        match builder.open() {
                            Ok(p) => {
                                port = Some(p);
                                let _ = evt_tx.send(RxEvent::Connected(port_name));
                            }
                            Err(e) => {
                                let _ = evt_tx.send(RxEvent::Error(format!("Open error: {e}")));
                            }
                        }
                    }
                    TxCmd::Disconnect => {
                        port = None;
                        let _ = evt_tx.send(RxEvent::Disconnected);
                    }
                    TxCmd::SendData(bytes) => {
                        if let Some(ref mut p) = port
                            && let Err(e) = p.write_all(&bytes)
                        {
                            let _ = evt_tx.send(RxEvent::Error(format!("Tx error: {e}")));
                        }
                    }
                    TxCmd::SendFile {
                        path,
                        chunk_size,
                        delay_ms,
                    } => {
                        if port.is_none() {
                            let _ = evt_tx.send(RxEvent::Error(
                                "Cannot send file: serial port is not connected".into(),
                            ));
                        } else {
                            match File::open(&path) {
                                Err(e) => {
                                    let _ = evt_tx.send(RxEvent::Error(format!(
                                        "Failed to open file '{}': {e}",
                                        path.display()
                                    )));
                                }
                                Ok(mut f) => {
                                    let total = f.metadata().map(|m| m.len() as usize).unwrap_or(0);
                                    let mut sent = 0;
                                    let effective_chunk = chunk_size.max(1);
                                    let mut chunk = vec![0u8; effective_chunk];
                                    let mut send_failed = false;

                                    while let Ok(n) = f.read(&mut chunk) {
                                        if n == 0 {
                                            break;
                                        }
                                        if let Some(ref mut p) = port {
                                            if let Err(e) = p.write_all(&chunk[..n]) {
                                                let _ = evt_tx.send(RxEvent::Error(format!(
                                                    "File transfer failed during write: {e}"
                                                )));
                                                send_failed = true;
                                                break;
                                            }
                                        } else {
                                            let _ = evt_tx.send(RxEvent::Error(
                                                "File transfer aborted: port disconnected".into(),
                                            ));
                                            send_failed = true;
                                            break;
                                        }

                                        sent += n;
                                        let _ = evt_tx.send(RxEvent::FileSendProgress { sent, total });
                                        ctx.request_repaint();

                                        // Drain incoming serial RX data during transfer to prevent buffer overrun
                                        if let Some(ref mut p) = port
                                            && let Ok(n_read) = p.read(&mut read_buf)
                                            && n_read > 0
                                        {
                                            let data = read_buf[..n_read].to_vec();
                                            if script_running.load(Ordering::Relaxed) {
                                                let _ = script_rx_tx.send(data.clone());
                                            }
                                            let _ = evt_tx.send(RxEvent::DataReceived(data));
                                            ctx.request_repaint();
                                        }

                                        // Process any disconnect command during file transfer
                                        if let Ok(cmd) = cmd_rx.try_recv()
                                            && matches!(cmd, TxCmd::Disconnect)
                                        {
                                            port = None;
                                            let _ = evt_tx.send(RxEvent::Disconnected);
                                            let _ = evt_tx.send(RxEvent::Error(
                                                "File transfer cancelled: user disconnected".into(),
                                            ));
                                            send_failed = true;
                                            break;
                                        }

                                        if delay_ms > 0 {
                                            thread::sleep(Duration::from_millis(delay_ms));
                                        }
                                    }

                                    if !send_failed {
                                        let _ = evt_tx.send(RxEvent::FileSendComplete);
                                    }
                                }
                            }
                        }
                    }
                    TxCmd::SetRts(val) => {
                        if let Some(ref mut p) = port {
                            let _ = p.write_request_to_send(val);
                        }
                    }
                    TxCmd::SetDtr(val) => {
                        if let Some(ref mut p) = port {
                            let _ = p.write_data_terminal_ready(val);
                        }
                    }
                }
                ctx.request_repaint();
            }

            if let Some(ref mut p) = port {
                match p.read(&mut read_buf) {
                    Ok(n) if n > 0 => {
                        let data = read_buf[..n].to_vec();
                        if script_running.load(Ordering::Relaxed) {
                            let _ = script_rx_tx.send(data.clone());
                        }
                        let _ = evt_tx.send(RxEvent::DataReceived(data));
                        ctx.request_repaint();
                    }
                    Ok(0) => {
                        port = None;
                        let _ = evt_tx.send(RxEvent::Error("Serial device disconnected (EOF)".into()));
                        let _ = evt_tx.send(RxEvent::Disconnected);
                        ctx.request_repaint();
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(e) => {
                        let _ = evt_tx.send(RxEvent::Error(format!("Read error: {e}")));
                        port = None;
                        let _ = evt_tx.send(RxEvent::Disconnected);
                        ctx.request_repaint();
                    }
                    _ => {}
                }
            } else {
                thread::sleep(Duration::from_millis(50));
            }
        }
    });
}

// ==========================================
// 4. Application State & GUI Engine
// ==========================================

pub struct SerialForgeApp {
    available_ports: Vec<String>,
    selected_port: String,
    baud_rate: u32,
    data_bits: DataBits,
    stop_bits: StopBits,
    parity: Parity,
    flow_control: FlowControl,

    rts: bool,
    dtr: bool,
    is_connected: bool,
    dark_mode: bool,

    cmd_tx: Sender<TxCmd>,
    evt_rx: Receiver<RxEvent>,
    script_rx_rx: Receiver<Vec<u8>>,

    rx_buffer: Vec<u8>,
    view_mode: ViewMode,
    auto_scroll: bool,
    //show_timestamps: bool,
    rx_bytes: usize,
    tx_bytes: usize,

    // Tools Tab Selection
    active_tab: ToolTab,

    // Sending & History
    send_text: String,
    line_ending: LineEnding,
    cmd_history: Vec<String>,
    cmd_history_idx: Option<usize>,

    // Preconfigured Commands
    preconfig_commands: Vec<PreconfigCommand>,
    new_preconfig_name: String,
    new_preconfig_cmd: String,

    // File Transfer
    file_path: Option<PathBuf>,
    chunk_size: usize,
    chunk_delay_ms: u64,
    file_progress: Option<(usize, usize)>,

    // Scripting Engine
    script_text: String,
    script_running: Arc<AtomicBool>,
    script_log: Arc<Mutex<Vec<String>>>,

    // System Logs
    system_log: Vec<String>,
    status_msg: String,
}

impl SerialForgeApp {
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

        let default_port = ports.first().cloned().unwrap_or_default();

        let mut app = Self {
            available_ports: ports,
            selected_port: default_port,
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            stop_bits: StopBits::One,
            parity: Parity::None,
            flow_control: FlowControl::None,
            rts: false,
            dtr: false,
            is_connected: false,
            dark_mode: false,
            cmd_tx,
            evt_rx,
            script_rx_rx,
            rx_buffer: Vec::new(),
            view_mode: ViewMode::Ascii,
            auto_scroll: true,
            // show_timestamps: true,
            rx_bytes: 0,
            tx_bytes: 0,
            active_tab: ToolTab::TransmitPresets,
            send_text: String::new(),
            line_ending: LineEnding::CRLF,
            cmd_history: Vec::new(),
            cmd_history_idx: None,
            preconfig_commands: vec![
                PreconfigCommand {
                    name: "Ping AT".into(),
                    cmd: "AT".into(),
                },
                PreconfigCommand {
                    name: "Device Info".into(),
                    cmd: "ATI".into(),
                },
                PreconfigCommand {
                    name: "Reset Device".into(),
                    cmd: "AT+RST".into(),
                },
                PreconfigCommand {
                    name: "Baud Check".into(),
                    cmd: "AT+BAUD?".into(),
                },
            ],
            new_preconfig_name: String::new(),
            new_preconfig_cmd: String::new(),
            file_path: None,
            chunk_size: 256,
            chunk_delay_ms: 10,
            file_progress: None,
            script_text: r#"// Example Serial Script (Rhai)
print("Sending Handshake...");
serial_send("AT\r\n");

let resp = serial_read_line(2000);
print("Received: " + resp);

if resp.contains("OK") {
    print("Device Ready!");
} else {
    print("No response or error.");
}
"#
            .to_string(),
            script_running,
            script_log: Arc::new(Mutex::new(Vec::new())),
            system_log: Vec::new(),
            status_msg: "Disconnected".to_string(),
        };

        app.log_system("Application initialized.");
        app
    }

    fn log_system(&mut self, msg: &str) {
        let time = Local::now().format("%H:%M:%S%.3f").to_string();
        self.system_log.push(format!("[{time}] {msg}"));
    }

    fn refresh_ports(&mut self) {
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
                    self.status_msg = format!("Connected to {port}");
                    self.log_system(&format!("Successfully connected to {port}"));
                }
                RxEvent::Disconnected => {
                    self.is_connected = false;
                    self.status_msg = "Disconnected".to_string();
                    self.log_system("Port disconnected.");
                }
                RxEvent::DataReceived(bytes) => {
                    self.rx_bytes += bytes.len();
                    self.rx_buffer.extend(bytes);
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
                }
            }
        }
    }

    fn send_string_command(&mut self, text: String) {
        if text.is_empty() || !self.is_connected {
            return;
        }

        if self.cmd_history.last() != Some(&text) {
            self.cmd_history.push(text.clone());
        }
        self.cmd_history_idx = None;

        let mut payload = text.clone().into_bytes();
        payload.extend_from_slice(self.line_ending.as_bytes());
        self.tx_bytes += payload.len();
        let _ = self.cmd_tx.send(TxCmd::SendData(payload));

        self.log_system(&format!("TX Command: \"{text}\""));
    }

    fn insert_script_text(&mut self, ctx: &egui::Context, editor_id: egui::Id, text: &str) {
        let byte_offset = |value: &str, char_index: usize| {
            value
                .char_indices()
                .nth(char_index)
                .map_or(value.len(), |(byte_index, _)| byte_index)
        };

        if let Some(state) = egui::TextEdit::load_state(ctx, editor_id)
            && let Some(range) = state.cursor.char_range()
        {
            let start = byte_offset(
                &self.script_text,
                range.primary.index.0.min(range.secondary.index.0),
            );
            let end = byte_offset(
                &self.script_text,
                range.primary.index.0.max(range.secondary.index.0),
            );
            self.script_text.replace_range(start..end, text);
        } else {
            self.script_text.push_str(text);
        }
    }

    fn run_script(&mut self, ctx: &egui::Context) {
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
                log_clone.lock().unwrap().push(s.to_string());
                ctx_print.request_repaint();
            });

            let cmd_tx_send = cmd_tx.clone();
            engine.register_fn("serial_send", move |s: &str| {
                let _ = cmd_tx_send.send(TxCmd::SendData(s.as_bytes().to_vec()));
            });

            let rx = script_rx_rx.clone();
            engine.register_fn("serial_read_line", move |timeout_ms: i64| -> String {
                let timeout = Duration::from_millis(u64::try_from(timeout_ms.max(0)).unwrap_or(0));
                let start = Instant::now();
                let mut accumulated = String::new();
                loop {
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

            if let Err(e) = engine.run(&script) {
                log.lock()
                    .unwrap()
                    .push(format!("[ERROR] Script error: {e}"));
            }

            is_running.store(false, Ordering::Relaxed);
            ctx_clone.request_repaint();
        });
    }
}

// ==========================================
// 5. egui UI Rendering Implementation
// ==========================================

impl eframe::App for SerialForgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        self.handle_events();

        // 1. TOP PANEL: Hardware Setup & Theme Switcher
        egui::Panel::top("top_panel").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                // ---------------------------------------------------------
                // GROUP 1: Connection & Port Selection
                // ---------------------------------------------------------
                ui.horizontal(|ui| {
                    if !self.is_connected {
                        if ui.button("🔌 Connect").clicked() {
                            let _ = self.cmd_tx.send(TxCmd::Connect {
                                port_name: self.selected_port.clone(),
                                baud_rate: self.baud_rate,
                                data_bits: self.data_bits,
                                stop_bits: self.stop_bits,
                                parity: self.parity,
                                flow_control: self.flow_control,
                            });
                        }
                    } else if ui.button("❌ Disconnect").clicked() {
                        let _ = self.cmd_tx.send(TxCmd::Disconnect);
                    }

                    ui.label("Port:");
                    egui::ComboBox::from_id_salt("port_combo")
                        .selected_text(&self.selected_port)
                        .show_ui(ui, |ui| {
                            for p in &self.available_ports {
                                ui.selectable_value(&mut self.selected_port, p.clone(), p);
                            }
                        });

                    if ui.button("🔄 R").clicked() {
                        self.refresh_ports();
                    }
                });

                ui.separator(); // Draws a vertical line between groups

                // ---------------------------------------------------------
                // GROUP 2: Baud Rate & Data Bits
                // ---------------------------------------------------------
                ui.horizontal(|ui| {
                    ui.label("Baud:");
                    egui::ComboBox::from_id_salt("baud_combo")
                        .width(0.0)
                        .selected_text(format!("{}", self.baud_rate))
                        .show_ui(ui, |ui| {
                            for b in [9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600] {
                                ui.selectable_value(&mut self.baud_rate, b, b.to_string());
                            }
                        });

                    ui.label("Data:");
                    egui::ComboBox::from_id_salt("data_combo")
                        .width(0.0)
                        .selected_text(format!("{}", self.data_bits))
                        .show_ui(ui, |ui| {
                            for b in [
                                DataBits::Five,
                                DataBits::Six,
                                DataBits::Seven,
                                DataBits::Eight,
                            ] {
                                ui.selectable_value(&mut self.data_bits, b, b.to_string());
                            }
                        });
                });

                ui.separator();

                // ---------------------------------------------------------
                // GROUP 3: Parity & Stop Bits
                // ---------------------------------------------------------
                ui.horizontal_centered(|ui| {
                    ui.label("Parity:");
                    egui::ComboBox::from_id_salt("parity_combo")
                        .width(0.0)
                        .selected_text(format!("{}", self.parity))
                        .show_ui(ui, |ui| {
                            for b in [Parity::None, Parity::Odd, Parity::Even] {
                                ui.selectable_value(&mut self.parity, b, b.to_string());
                            }
                        });

                    ui.label("Bits:");
                    egui::ComboBox::from_id_salt("bits_combo")
                        .width(0.0)
                        .selected_text(format!("{}", self.stop_bits))
                        .show_ui(ui, |ui| {
                            for b in [StopBits::One, StopBits::Two] {
                                ui.selectable_value(&mut self.stop_bits, b, b.to_string());
                            }
                        });
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Flow:");
                    egui::ComboBox::from_id_salt("flow_control_combo")
                        .width(0.0)
                        .selected_text(format!("{}", self.flow_control))
                        .show_ui(ui, |ui| {
                            for b in [
                                FlowControl::None,
                                FlowControl::Software,
                                FlowControl::Hardware,
                            ] {
                                ui.selectable_value(&mut self.flow_control, b, b.to_string());
                            }
                        });
                });
                ui.separator();

                // ---------------------------------------------------------
                // GROUP 4: Hardware Control Signals
                // ---------------------------------------------------------
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.rts, "RTS").changed() && self.is_connected {
                        let _ = self.cmd_tx.send(TxCmd::SetRts(self.rts));
                    }
                    if ui.checkbox(&mut self.dtr, "DTR").changed() && self.is_connected {
                        let _ = self.cmd_tx.send(TxCmd::SetDtr(self.dtr));
                    }
                });

                // ---------------------------------------------------------
                // GROUP 5: Right-Aligned Theme Toggle
                // ---------------------------------------------------------
                // This pushes the theme button to the far right of the available space
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let theme_btn = if self.dark_mode {
                        "🌙 Dark"
                    } else {
                        "☀️ Light"
                    };
                    if ui.button(theme_btn).clicked() {
                        self.dark_mode = !self.dark_mode;
                    }
                });
            });
        });

        // 2. BOTTOM PANEL: System Status Summary
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Status: {}", self.status_msg));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "Tx: {} bytes | Rx: {} bytes",
                        self.tx_bytes, self.rx_bytes
                    ));
                });
            });
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
                    ui.horizontal(|ui| {
                        ui.label("Send String:");
                        let text_entry = ui.add(
                            egui::TextEdit::singleline(&mut self.send_text)
                                .desired_width(280.0)
                                .hint_text("Type command or use ↑/↓ for history"),
                        );

                        if text_entry.has_focus() {
                            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                                if !self.cmd_history.is_empty() {
                                    let next_idx = match self.cmd_history_idx {
                                        None => self.cmd_history.len().saturating_sub(1),
                                        Some(idx) => idx.saturating_sub(1),
                                    };
                                    self.cmd_history_idx = Some(next_idx);
                                    self.send_text = self.cmd_history[next_idx].clone();
                                }
                            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowDown))
                                && let Some(idx) = self.cmd_history_idx
                            {
                                if idx + 1 < self.cmd_history.len() {
                                    let next_idx = idx + 1;
                                    self.cmd_history_idx = Some(next_idx);
                                    self.send_text = self.cmd_history[next_idx].clone();
                                } else {
                                    self.cmd_history_idx = None;
                                    self.send_text.clear();
                                }
                            }
                        }

                        egui::ComboBox::from_id_salt("cmd_history_combo")
                            .selected_text("📜 History")
                            .show_ui(ui, |ui| {
                                if self.cmd_history.is_empty() {
                                    ui.label("No history yet");
                                } else {
                                    for h in self.cmd_history.iter().rev() {
                                        if ui.selectable_label(false, h).clicked() {
                                            self.send_text = h.clone();
                                        }
                                    }
                                }
                            });

                        ui.label("EOL:");
                        egui::ComboBox::from_id_salt("line_ending_combo")
                            .selected_text(match self.line_ending {
                                LineEnding::None => "None",
                                LineEnding::Null => "NULL (0x00)",
                                LineEnding::CR => "CR (\\r)",
                                LineEnding::LF => "LF (\\n)",
                                LineEnding::CRLF => "CRLF (\\r\\n)",
                                LineEnding::Etx => "ETX (0x03)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.line_ending,
                                    LineEnding::None,
                                    "None",
                                );
                                ui.selectable_value(
                                    &mut self.line_ending,
                                    LineEnding::Null,
                                    "NULL (0x00)",
                                );
                                ui.selectable_value(
                                    &mut self.line_ending,
                                    LineEnding::CR,
                                    "CR (\\r)",
                                );
                                ui.selectable_value(
                                    &mut self.line_ending,
                                    LineEnding::LF,
                                    "LF (\\n)",
                                );
                                ui.selectable_value(
                                    &mut self.line_ending,
                                    LineEnding::CRLF,
                                    "CRLF (\\r\\n)",
                                );
                                ui.selectable_value(
                                    &mut self.line_ending,
                                    LineEnding::Etx,
                                    "ETX (0x03)",
                                );
                            });

                        let trigger_send = ui.button("Send Text").clicked()
                            || (text_entry.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)));

                        if trigger_send {
                            let text = self.send_text.clone();
                            self.send_string_command(text);
                            self.send_text.clear();
                        }
                    });

                    ui.separator();

                    ui.label(egui::RichText::new("⭐ Preset Command Macros").strong());
                    let mut to_send = None;
                    let mut to_remove = None;

                    egui::Grid::new("preconfig_grid")
                        .striped(true)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Preset Name").strong());
                            ui.label(egui::RichText::new("Command String").strong());
                            ui.label(egui::RichText::new("Actions").strong());
                            ui.end_row();

                            for (idx, item) in self.preconfig_commands.iter().enumerate() {
                                ui.label(&item.name);
                                ui.monospace(&item.cmd);

                                ui.horizontal(|ui| {
                                    if ui.button("▶ Send").clicked() {
                                        to_send = Some(item.cmd.clone());
                                    }
                                    if ui.button("📋 Load").clicked() {
                                        self.send_text = item.cmd.clone();
                                    }
                                    if ui.button("🗑").clicked() {
                                        to_remove = Some(idx);
                                    }
                                });
                                ui.end_row();
                            }
                        });

                    if let Some(cmd) = to_send {
                        self.send_string_command(cmd);
                    }
                    if let Some(idx) = to_remove {
                        self.preconfig_commands.remove(idx);
                    }

                    ui.horizontal(|ui| {
                        ui.label("New Preset:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_preconfig_name)
                                .hint_text("Name")
                                .desired_width(120.0),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_preconfig_cmd)
                                .hint_text("Command String")
                                .desired_width(220.0),
                        );

                        if ui.button("➕ Add Preset").clicked()
                            && !self.new_preconfig_cmd.is_empty()
                        {
                            let name = if self.new_preconfig_name.is_empty() {
                                self.new_preconfig_cmd.clone()
                            } else {
                                self.new_preconfig_name.clone()
                            };
                            self.preconfig_commands.push(PreconfigCommand {
                                name,
                                cmd: self.new_preconfig_cmd.clone(),
                            });
                            self.new_preconfig_name.clear();
                            self.new_preconfig_cmd.clear();
                        }
                    });
                }
                ToolTab::BinaryFile => {
                    ui.horizontal(|ui| {
                        ui.label("Binary Payload:");
                        if ui.button("📁 Select File").clicked()
                            && let Some(path) = FileDialog::new().pick_file()
                        {
                            self.file_path = Some(path);
                        }

                        if let Some(ref path) = self.file_path {
                            ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                        }

                        ui.add(egui::DragValue::new(&mut self.chunk_size).prefix("Chunk size: "));
                        ui.add(egui::DragValue::new(&mut self.chunk_delay_ms).prefix("Delay ms: "));

                        if ui.button("📤 Transmit File").clicked()
                            && self.is_connected
                            && let Some(ref path) = self.file_path
                        {
                            let _ = self.cmd_tx.send(TxCmd::SendFile {
                                path: path.clone(),
                                chunk_size: self.chunk_size,
                                delay_ms: self.chunk_delay_ms,
                            });
                        }
                    });

                    if let Some((sent, total)) = self.file_progress {
                        let progress = sent as f32 / total as f32;
                        ui.add(
                            egui::ProgressBar::new(progress).text(format!("{sent}/{total} Bytes")),
                        );
                    }
                }
                ToolTab::Scripting => {
                    ui.horizontal(|ui| {
                        if ui.button("▶ Run Script").clicked() && self.is_connected {
                            self.run_script(&ctx);
                        }
                        if ui.button("📂 Open Script").clicked()
                            && let Some(path) = FileDialog::new()
                                .add_filter("Rhai scripts", &["rhai"])
                                .pick_file()
                        {
                            match std::fs::read_to_string(&path) {
                                Ok(script) => {
                                    self.script_text = script;
                                    self.status_msg = format!(
                                        "Loaded script: {}",
                                        path.file_name().unwrap_or_default().to_string_lossy()
                                    );
                                }
                                Err(error) => {
                                    self.status_msg = format!("Script load failed: {error}");
                                    self.log_system(&self.status_msg.clone());
                                }
                            }
                        }
                        if self.script_running.load(Ordering::Relaxed) {
                            ui.spinner();
                            ui.label("Script executing...");
                        }
                    });

                    ui.columns(2, |cols| {
                        let ctx = cols[0].ctx().clone();
                        let editor_id = egui::Id::new("rhai_script_editor");
                        let paste_requested = cols[0]
                            .horizontal(|ui| {
                                let paste_requested = ui.button("📋 Paste").clicked();
                                ui.label("Rhai Script Editor:");
                                paste_requested
                            })
                            .inner;
                        let editor_has_focus = ctx.memory(|memory| memory.has_focus(editor_id));
                        let key_paste_requested = editor_has_focus
                            && ctx.input(|input| {
                                input.key_pressed(egui::Key::V) && input.modifiers.command
                            });

                        let layouter =
                            &mut |ui: &egui::Ui,
                                  string: &dyn egui::TextBuffer,
                                  _wrap_width: f32| {
                                ui.painter()
                                    .layout_job(highlight_rhai_code(ui, string.as_str()))
                            };

                        let mut paste_text: Option<String> = if editor_has_focus {
                            ctx.input(|input| {
                                input.events.iter().find_map(|event| match event {
                                    egui::Event::Paste(text) => Some(text.clone()),
                                    _ => None,
                                })
                            })
                        } else {
                            None
                        };

                        if paste_text.is_none() && (paste_requested || key_paste_requested) {
                            match arboard::Clipboard::new()
                                .and_then(|mut clipboard| clipboard.get_text())
                            {
                                Ok(text) => paste_text = Some(text),
                                Err(error) => {
                                    self.log_system(&format!("Clipboard paste failed: {error}"))
                                }
                            }
                        }

                        if let Some(text) = paste_text {
                            ctx.input_mut(|input| {
                                input
                                    .events
                                    .retain(|event| !matches!(event, egui::Event::Paste(_)));
                            });
                            self.insert_script_text(&ctx, editor_id, &text);
                        }

                        let editor_width = cols[0].available_width();
                        let editor_response = scrollable_script_field(
                            &mut cols[0],
                            "rhai_editor_scroll",
                            egui::TextEdit::multiline(&mut self.script_text)
                                .id(editor_id)
                                .font(egui::TextStyle::Monospace)
                                .layouter(layouter)
                                .desired_width(editor_width),
                        )
                        .inner;

                        if paste_requested {
                            editor_response.request_focus();
                        }

                        cols[1].label("Script Log:");
                        let mut logs = self.script_log.lock().unwrap().join("\n");
                        let log_width = cols[1].available_width();
                        scrollable_script_field(
                            &mut cols[1],
                            "rhai_log_scroll",
                            egui::TextEdit::multiline(&mut logs)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(log_width),
                        );
                    });
                }
                ToolTab::SystemLog => {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("System Activity Console").strong());
                        if ui.button("🗑 Clear Log").clicked() {
                            self.system_log.clear();
                        }
                    });

                    let mut logs = self.system_log.join("\n");
                    ui.add(
                        egui::TextEdit::multiline(&mut logs)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY),
                    );
                }
            }
        });

        // 4. CENTRAL PANEL: Primary Serial Terminal Display
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.view_mode, ViewMode::Ascii, "ASCII");
                ui.selectable_value(&mut self.view_mode, ViewMode::Hex, "HEX");
                ui.selectable_value(&mut self.view_mode, ViewMode::Mixed, "Mixed");

                ui.separator();
                ui.checkbox(&mut self.auto_scroll, "Auto-Scroll");
                // TODO
                // ui.checkbox(&mut self.show_timestamps, "Timestamps");

                if ui.button("🗑 Clear Console").clicked() {
                    self.rx_buffer.clear();
                }
            });

            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(self.auto_scroll)
                .show(ui, |ui| {
                    let mut formatted_text = match self.view_mode {
                        ViewMode::Ascii => String::from_utf8_lossy(&self.rx_buffer).to_string(),
                        ViewMode::Hex => self
                            .rx_buffer
                            .chunks(16)
                            .map(|chunk| {
                                chunk
                                    .iter()
                                    .map(|b| format!("{b:02X}"))
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            })
                            .collect::<Vec<_>>()
                            .join("\n"),
                        ViewMode::Mixed => self
                            .rx_buffer
                            .chunks(16)
                            .map(|chunk| {
                                let hex = chunk
                                    .iter()
                                    .map(|b| format!("{b:02X}"))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                let ascii: String = chunk
                                    .iter()
                                    .map(|&b| {
                                        if b.is_ascii_graphic() || b == b' ' {
                                            b as char
                                        } else {
                                            '.'
                                        }
                                    })
                                    .collect();
                                format!("{hex:<48} | {ascii}")
                            })
                            .collect::<Vec<_>>()
                            .join("\n"),
                    };

                    ui.add(
                        egui::TextEdit::multiline(&mut formatted_text)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .lock_focus(true),
                    );
                });
        });
    }
}

// ==========================================
// 6. App Entry Point
// ==========================================

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 760.0])
            .with_title("SerialForge - Cross-Platform Serial Terminal"),
        ..Default::default()
    };

    eframe::run_native(
        "SerialForge Terminal",
        native_options,
        Box::new(|cc| Ok(Box::new(SerialForgeApp::new(cc)))),
    )
}

#[cfg(test)]
mod editor_tests {
    use super::*;

    #[test]
    fn full_tactical_rpg_script_has_bounded_scroll_viewport() {
        let source = include_str!("../scripts/tactical_rpg.rhai");
        for size in [egui::vec2(320.0, 180.0), egui::vec2(640.0, 400.0)] {
            let ctx = egui::Context::default();
            let mut text = source.to_owned();
            for _frame in 0..3 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    ..Default::default()
                };
                ctx.run_ui(input, |ui| {
                    let available = ui.available_size();
                    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _: f32| {
                        ui.painter()
                            .layout_job(highlight_rhai_code(ui, text.as_str()))
                    };
                    let output = scrollable_script_field(
                        ui,
                        "test_script_scroll",
                        egui::TextEdit::multiline(&mut text)
                            .layouter(&mut layouter)
                            .desired_width(available.x),
                    );
                    assert!(output.inner_rect.height() <= available.y);
                    assert!(output.inner_rect.width() <= available.x);
                    assert!(output.content_size.y > output.inner_rect.height());
                    assert!(ui.min_rect().height() <= available.y + 1.0);
                })
                .drop_without_applying_deltas();
            }
            assert_eq!(text, source);
        }
    }
}
