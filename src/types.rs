use serialport::{
    DataBits,
    FlowControl,
    Parity,
    StopBits,
};
use std::path::PathBuf;

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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ViewMode {
    Ascii,
    Hex,
    Mixed,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LineEnding {
    None,
    Null,
    CR,
    LF,
    Crlf,
    Etx,
}

impl LineEnding {
    #[must_use]
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            LineEnding::None => b"",
            LineEnding::Null => b"\x00",
            LineEnding::CR => b"\r",
            LineEnding::LF => b"\n",
            LineEnding::Crlf => b"\r\n",
            LineEnding::Etx => b"\x03",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreconfigCommand {
    pub name: String,
    pub cmd: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ToolTab {
    TransmitPresets,
    BinaryFile,
    Scripting,
    SystemLog,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_ending_bytes() {
        assert_eq!(LineEnding::None.as_bytes(), b"");
        assert_eq!(LineEnding::Null.as_bytes(), b"\x00");
        assert_eq!(LineEnding::CR.as_bytes(), b"\r");
        assert_eq!(LineEnding::LF.as_bytes(), b"\n");
        assert_eq!(LineEnding::Crlf.as_bytes(), b"\r\n");
        assert_eq!(LineEnding::Etx.as_bytes(), b"\x03");
    }
}
