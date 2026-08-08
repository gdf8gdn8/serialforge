use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui;
use rfd::FileDialog;
use rhai::{Engine, Scope, AST};
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

// ==========================================
// 1. Thread Communication Data Types
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
    CR,
    LF,
    CRLF,
}

impl LineEnding {
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            LineEnding::None => b"",
            LineEnding::CR => b"\r",
            LineEnding::LF => b"\n",
            LineEnding::CRLF => b"\r\n",
        }
    }
}

// ==========================================
// 2. Serial Worker Thread (I/O Loop)
// ==========================================

fn spawn_serial_worker(
    cmd_rx: Receiver<TxCmd>,
    evt_tx: Sender<RxEvent>,
    ctx: egui::Context,
    script_rx_tx: Sender<Vec<u8>>,
) {
    thread::spawn(move || {
        let mut port: Option<Box<dyn SerialPort>> = None;
        let mut read_buf = vec![0u8; 4096];

        loop {
            // Process non-blocking commands from UI
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
                        if let Some(ref mut p) = port {
                            if let Err(e) = p.write_all(&bytes) {
                                let _ = evt_tx.send(RxEvent::Error(format!("Tx error: {e}")));
                            }
                        }
                    }
                    TxCmd::SendFile {
                        path,
                        chunk_size,
                        delay_ms,
                    } => {
                        if let Some(ref mut p) = port {
                            if let Ok(mut f) = File::open(&path) {
                                let total = f.metadata().map(|m| m.len() as usize).unwrap_or(0);
                                let mut sent = 0;
                                let mut chunk = vec![0u8; chunk_size];

                                while let Ok(n) = f.read(&mut chunk) {
                                    if n == 0 {
                                        break;
                                    }
                                    if p.write_all(&chunk[..n]).is_err() {
                                        let _ = evt_tx.send(RxEvent::Error("File transfer failed during write".into()));
                                        break;
                                    }
                                    sent += n;
                                    let _ = evt_tx.send(RxEvent::FileSendProgress { sent, total });
                                    ctx.request_repaint();
                                    if delay_ms > 0 {
                                        thread::sleep(Duration::from_millis(delay_ms));
                                    }
                                }
                                let _ = evt_tx.send(RxEvent::FileSendComplete);
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

            // Read available data from serial hardware
            if let Some(ref mut p) = port {
                match p.read(&mut read_buf) {
                    Ok(n) if n > 0 => {
                        let data = read_buf[..n].to_vec();
                        let _ = script_rx_tx.send(data.clone());
                        let _ = evt_tx.send(RxEvent::DataReceived(data));
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
// 3. Application State & GUI Engine
// ==========================================

pub struct HTermApp {
    // Port Selection & Settings
    available_ports: Vec<String>,
    selected_port: String,
    baud_rate: u32,
    data_bits: DataBits,
    stop_bits: StopBits,
    parity: Parity,
    flow_control: FlowControl,

    // Controls
    rts: bool,
    dtr: bool,
    is_connected: bool,

    // Channels
    cmd_tx: Sender<TxCmd>,
    evt_rx: Receiver<RxEvent>,
    script_rx_rx: Receiver<Vec<u8>>,

    // Buffer & Display
    rx_buffer: Vec<u8>,
    view_mode: ViewMode,
    auto_scroll: bool,
    show_timestamps: bool,
    rx_bytes: usize,
    tx_bytes: usize,

    // Sending
    send_text: String,
    send_hex: String,
    line_ending: LineEnding,

    // Binary File Transfer
    file_path: Option<PathBuf>,
    chunk_size: usize,
    chunk_delay_ms: u64,
    file_progress: Option<(usize, usize)>,

    // Scripting Engine
    script_text: String,
    script_running: Arc<AtomicBool>,
    script_log: Arc<Mutex<Vec<String>>>,

    // Status
    status_msg: String,
}

impl HTermApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (cmd_tx, cmd_rx) = unbounded();
        let (evt_tx, evt_rx) = unbounded();
        let (script_rx_tx, script_rx_rx) = unbounded();

        spawn_serial_worker(cmd_rx, evt_tx, cc.egui_ctx.clone(), script_rx_tx);

        let ports = serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.port_name)
            .collect::<Vec<_>>();

        let default_port = ports.first().cloned().unwrap_or_default();

        Self {
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
            cmd_tx,
            evt_rx,
            script_rx_rx,
            rx_buffer: Vec::new(),
            view_mode: ViewMode::Ascii,
            auto_scroll: true,
            show_timestamps: true,
            rx_bytes: 0,
            tx_bytes: 0,
            send_text: String::new(),
            send_hex: String::new(),
            line_ending: LineEnding::CRLF,
            file_path: None,
            chunk_size: 256,
            chunk_delay_ms: 10,
            file_progress: None,
            script_text: r#"// Example Serial Script (Rhai)
print("Sending Handshake...");
serial_send("AT\r\n");

// Wait for OK response
let resp = serial_read_line(2000);
print("Received: " + resp);

if resp.contains("OK") {
    print("Device Ready!");
} else {
    print("No response or error.");
}
"#.to_string(),
            script_running: Arc::new(AtomicBool::new(false)),
            script_log: Arc::new(Mutex::new(Vec::new())),
            status_msg: "Disconnected".to_string(),
        }
    }

    fn refresh_ports(&mut self) {
        if let Ok(ports) = serialport::available_ports() {
            self.available_ports = ports.into_iter().map(|p| p.port_name).collect();
            if !self.available_ports.contains(&self.selected_port) && !self.available_ports.is_empty() {
                self.selected_port = self.available_ports[0].clone();
            }
        }
    }

    fn handle_events(&mut self) {
        while let Ok(evt) = self.evt_rx.try_recv() {
            match evt {
                RxEvent::Connected(port) => {
                    self.is_connected = true;
                    self.status_msg = format!("Connected to {port}");
                }
                RxEvent::Disconnected => {
                    self.is_connected = false;
                    self.status_msg = "Disconnected".to_string();
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
                }
                RxEvent::Error(e) => {
                    self.status_msg = format!("Error: {e}");
                }
            }
        }
    }

    fn run_script(&self) {
        if self.script_running.load(Ordering::Relaxed) {
            return;
        }

        let script = self.script_text.clone();
        let cmd_tx = self.cmd_tx.clone();
        let script_rx_rx = self.script_rx_rx.clone();
        let is_running = self.script_running.clone();
        let log = self.script_log.clone();

        is_running.store(true, Ordering::Relaxed);
        log.lock().unwrap().clear();

        thread::spawn(move || {
            let mut engine = Engine::new();

            // Bind print script logs
            let log_clone = log.clone();
            engine.on_print(move |s| {
                log_clone.lock().unwrap().push(s.to_string());
            });

            // Bind serial_send function
            let cmd_tx_send = cmd_tx.clone();
            engine.register_fn("serial_send", move |s: &str| {
                let _ = cmd_tx_send.send(TxCmd::SendData(s.as_bytes().to_vec()));
            });

            // Bind serial_read_line with timeout
            let rx = script_rx_rx.clone();
            engine.register_fn("serial_read_line", move |timeout_ms: i64| -> String {
                let start = Instant::now();
                let mut accumulated = String::new();
                loop {
                    if start.elapsed() > Duration::from_millis(timeout_ms as u64) {
                        break;
                    }
                    if let Ok(bytes) = rx.try_recv() {
                        let text = String::from_utf8_lossy(&bytes);
                        accumulated.push_str(&text);
                        if accumulated.contains('\n') {
                            break;
                        }
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                accumulated
            });

            // Run Rhai Script
            if let Err(e) = engine.run(&script) {
                log.lock().unwrap().push(format!("[ERROR] Script error: {e}"));
            }

            is_running.store(false, Ordering::Relaxed);
        });
    }
}

// ==========================================
// 4. egui UI Rendering Implementation
// ==========================================

impl eframe::App for HTermApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events();

        // Top Command Panel: Port Connection Parameters
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("HTerm-rs Serial Terminal");
                ui.separator();

                if ui.button("🔄 Refresh").clicked() {
                    self.refresh_ports();
                }

                egui::ComboBox::from_label("Port")
                    .selected_text(&self.selected_port)
                    .show_ui(ui, |ui| {
                        for p in &self.available_ports {
                            ui.selectable_value(&mut self.selected_port, p.clone(), p);
                        }
                    });

                egui::ComboBox::from_label("Baud")
                    .selected_text(format!("{}", self.baud_rate))
                    .show_ui(ui, |ui| {
                        for b in [9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600] {
                            ui.selectable_value(&mut self.baud_rate, b, b.to_string());
                        }
                    });

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

                ui.separator();
                if ui.checkbox(&mut self.rts, "RTS").changed() && self.is_connected {
                    let _ = self.cmd_tx.send(TxCmd::SetRts(self.rts));
                }
                if ui.checkbox(&mut self.dtr, "DTR").changed() && self.is_connected {
                    let _ = self.cmd_tx.send(TxCmd::SetDtr(self.dtr));
                }
            });
        });

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Status: {}", self.status_msg));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Tx: {} bytes | Rx: {} bytes", self.tx_bytes, self.rx_bytes));
                });
            });
        });

        // Central Main Control Interface
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // View Mode Selectors
                ui.selectable_value(&mut self.view_mode, ViewMode::Ascii, "ASCII");
                ui.selectable_value(&mut self.view_mode, ViewMode::Hex, "HEX");
                ui.selectable_value(&mut self.view_mode, ViewMode::Mixed, "Mixed");

                ui.separator();
                ui.checkbox(&mut self.auto_scroll, "Auto-Scroll");
                ui.checkbox(&mut self.show_timestamps, "Timestamps");

                if ui.button("🗑 Clear Console").clicked() {
                    self.rx_buffer.clear();
                }
            });

            ui.separator();

            // Terminal Console Screen
            let text_style = egui::TextStyle::Monospace;
            let row_height = ui.text_style_height(&text_style);

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
                                    .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
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

            ui.separator();

            // Tabs for Transmitting Text, File, or Script
            egui::CollapsingHeader::new("🚀 Data Transmission & Tools")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Text Transmission
                        ui.label("Send String:");
                        let text_entry = ui.text_edit_singleline(&mut self.send_text);
                        
                        egui::ComboBox::from_id_source("line_ending")
                            .selected_text(match self.line_ending {
                                LineEnding::None => "None",
                                LineEnding::CR => "CR (\\r)",
                                LineEnding::LF => "LF (\\n)",
                                LineEnding::CRLF => "CRLF (\\r\\n)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.line_ending, LineEnding::None, "None");
                                ui.selectable_value(&mut self.line_ending, LineEnding::CR, "CR (\\r)");
                                ui.selectable_value(&mut self.line_ending, LineEnding::LF, "LF (\\n)");
                                ui.selectable_value(&mut self.line_ending, LineEnding::CRLF, "CRLF (\\r\\n)");
                            });

                        if (ui.button("Send Text").clicked() || (text_entry.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))) && self.is_connected {
                            let mut payload = self.send_text.as_bytes().to_vec();
                            payload.extend_from_slice(self.line_ending.as_bytes());
                            self.tx_bytes += payload.len();
                            let _ = self.cmd_tx.send(TxCmd::SendData(payload));
                            self.send_text.clear();
                        }
                    });

                    ui.separator();

                    // Binary File Transfer Feature
                    ui.horizontal(|ui| {
                        ui.label("Binary File:");
                        if ui.button("📁 Select File").clicked() {
                            if let Some(path) = FileDialog::new().pick_file() {
                                self.file_path = Some(path);
                            }
                        }

                        if let Some(ref path) = self.file_path {
                            ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                        }

                        ui.add(egui::DragValue::new(&mut self.chunk_size).prefix("Chunk Size: "));
                        ui.add(egui::DragValue::new(&mut self.chunk_delay_ms).prefix("Delay (ms): "));

                        if ui.button("📤 Transmit Binary File").clicked() && self.is_connected {
                            if let Some(ref path) = self.file_path {
                                let _ = self.cmd_tx.send(TxCmd::SendFile {
                                    path: path.clone(),
                                    chunk_size: self.chunk_size,
                                    delay_ms: self.chunk_delay_ms,
                                });
                            }
                        }
                    });

                    if let Some((sent, total)) = self.file_progress {
                        let progress = sent as f32 / total as f32;
                        ui.add(egui::ProgressBar::new(progress).text(format!("{sent}/{total} Bytes")));
                    }

                    ui.separator();

                    // Rhai Script Engine Integration
                    ui.collapsing("📜 Embedded Scripting Engine (Rhai)", |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("▶ Run Script").clicked() && self.is_connected {
                                self.run_script();
                            }
                            if self.script_running.load(Ordering::Relaxed) {
                                ui.spinner();
                                ui.label("Script executing...");
                            }
                        });

                        ui.columns(2, |cols| {
                            cols[0].label("Rhai Script:");
                            cols[0].add(
                                egui::TextEdit::multiline(&mut self.script_text)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(8)
                                    .desired_width(f32::INFINITY),
                            );

                            cols[1].label("Script Execution Log:");
                           let mut logs = self.script_log.lock().unwrap().join("\n");
                            cols[1].add(
    egui::TextEdit::multiline(&mut logs)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(8)
                                    .desired_width(f32::INFINITY),
                            );
                        });
                    });
                });
        });
    }
}

// ==========================================
// 5. App Entry Point
// ==========================================

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_title("HTerm-rs - Cross-Platform Serial Terminal"),
        ..Default::default()
    };

    eframe::run_native(
        "HTerm-rs Terminal",
        native_options,
        Box::new(|cc| Ok(Box::new(HTermApp::new(cc)))),
    )
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