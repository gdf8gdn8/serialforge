use crate::types::{
    LineEnding,
    PreconfigCommand,
    ToolTab,
    ViewMode,
};
use serde::{
    Deserialize,
    Serialize,
};

pub const MAX_SYSTEM_LOG_ENTRIES: usize = 2000;
pub const MAX_SCRIPT_LOG_ENTRIES: usize = 2000;
pub const MAX_RX_BUFFER_SIZE: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub selected_port: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub parity: String,
    pub flow_control: String,
    pub auto_reconnect: bool,
    pub dark_mode: bool,
    pub view_mode: ViewMode,
    pub auto_scroll: bool,
    pub active_tab: ToolTab,
    pub line_ending: LineEnding,
    pub preconfig_commands: Vec<PreconfigCommand>,
    pub chunk_size: usize,
    pub chunk_delay_ms: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            selected_port: String::new(),
            baud_rate: 115_200,
            data_bits: 8,
            stop_bits: 1,
            parity: "None".to_string(),
            flow_control: "None".to_string(),
            auto_reconnect: false,
            dark_mode: false,
            view_mode: ViewMode::Ascii,
            auto_scroll: true,
            active_tab: ToolTab::TransmitPresets,
            line_ending: LineEnding::Crlf,
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
            ],
            chunk_size: 256,
            chunk_delay_ms: 10,
        }
    }
}
