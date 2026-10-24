use crate::app::SerialForgeApp;
use crate::types::TxCmd;
use eframe::egui;
use serialport::{
    DataBits,
    FlowControl,
    Parity,
    StopBits,
};

pub fn render_toolbar(app: &mut SerialForgeApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        // GROUP 1: Connection & Port Selection
        ui.horizontal(|ui| {
            if !app.is_connected {
                if ui.button("🔌 Connect").clicked() {
                    let _ = app.cmd_tx.send(TxCmd::Connect {
                        port_name: app.selected_port.clone(),
                        baud_rate: app.baud_rate,
                        data_bits: app.data_bits,
                        stop_bits: app.stop_bits,
                        parity: app.parity,
                        flow_control: app.flow_control,
                    });
                }
            } else if ui.button("❌ Disconnect").clicked() {
                let _ = app.cmd_tx.send(TxCmd::Disconnect);
            }

            ui.label("Port:");
            egui::ComboBox::from_id_salt("port_combo")
                .selected_text(&app.selected_port)
                .show_ui(ui, |ui| {
                    for p in &app.available_ports {
                        ui.selectable_value(&mut app.selected_port, p.clone(), p);
                    }
                });

            if ui.button("🔄 R").clicked() {
                app.refresh_ports();
            }
        });

        ui.separator();

        // GROUP 2: Baud Rate & Data Bits
        ui.horizontal(|ui| {
            ui.label("Baud:");
            egui::ComboBox::from_id_salt("baud_combo")
                .width(0.0)
                .selected_text(format!("{}", app.baud_rate))
                .show_ui(ui, |ui| {
                    for b in [
                        9600, 19200, 38400, 57600, 115_200, 230_400, 460_800, 921_600,
                    ] {
                        ui.selectable_value(&mut app.baud_rate, b, b.to_string());
                    }
                });

            ui.label("Data:");
            egui::ComboBox::from_id_salt("data_combo")
                .width(0.0)
                .selected_text(format!("{}", app.data_bits))
                .show_ui(ui, |ui| {
                    for b in [
                        DataBits::Five,
                        DataBits::Six,
                        DataBits::Seven,
                        DataBits::Eight,
                    ] {
                        ui.selectable_value(&mut app.data_bits, b, b.to_string());
                    }
                });
        });

        ui.separator();

        // GROUP 3: Parity & Stop Bits & Flow Control
        ui.horizontal_centered(|ui| {
            ui.label("Parity:");
            egui::ComboBox::from_id_salt("parity_combo")
                .width(0.0)
                .selected_text(format!("{}", app.parity))
                .show_ui(ui, |ui| {
                    for b in [Parity::None, Parity::Odd, Parity::Even] {
                        ui.selectable_value(&mut app.parity, b, b.to_string());
                    }
                });

            ui.label("Bits:");
            egui::ComboBox::from_id_salt("bits_combo")
                .width(0.0)
                .selected_text(format!("{}", app.stop_bits))
                .show_ui(ui, |ui| {
                    for b in [StopBits::One, StopBits::Two] {
                        ui.selectable_value(&mut app.stop_bits, b, b.to_string());
                    }
                });
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Flow:");
            egui::ComboBox::from_id_salt("flow_control_combo")
                .width(0.0)
                .selected_text(format!("{}", app.flow_control))
                .show_ui(ui, |ui| {
                    for b in [
                        FlowControl::None,
                        FlowControl::Software,
                        FlowControl::Hardware,
                    ] {
                        ui.selectable_value(&mut app.flow_control, b, b.to_string());
                    }
                });
        });
        ui.separator();

        // GROUP 4: Hardware Control Signals
        ui.horizontal(|ui| {
            if ui.checkbox(&mut app.rts, "RTS").changed() && app.is_connected {
                let _ = app.cmd_tx.send(TxCmd::SetRts(app.rts));
            }
            if ui.checkbox(&mut app.dtr, "DTR").changed() && app.is_connected {
                let _ = app.cmd_tx.send(TxCmd::SetDtr(app.dtr));
            }
        });

        // GROUP 5: Right-Aligned Theme Toggle
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let theme_btn = if app.dark_mode {
                "🌙 Dark"
            } else {
                "☀️ Light"
            };
            if ui.button(theme_btn).clicked() {
                app.dark_mode = !app.dark_mode;
            }
        });
    });
}
