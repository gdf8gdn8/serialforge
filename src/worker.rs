use crate::types::{
    RxEvent,
    TxCmd,
};
use crossbeam_channel::{
    Receiver,
    Sender,
};
use eframe::egui;
use serialport::SerialPort;
use std::fs::File;
use std::io::{
    Read,
    Write,
};
use std::sync::Arc;
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};
use std::thread;
use std::time::Duration;

pub fn spawn_serial_worker(
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
                                        let _ =
                                            evt_tx.send(RxEvent::FileSendProgress { sent, total });
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
                        let _ =
                            evt_tx.send(RxEvent::Error("Serial device disconnected (EOF)".into()));
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
