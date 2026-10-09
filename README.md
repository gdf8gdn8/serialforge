# serialforge

A fast, modern, cross-platform serial terminal inspired by **HTerm** and **SerialGUI-rs**. Built with Rust, featuring an **`egui`** hardware-accelerated GUI, **`serialport-rs`** for low-level hardware communication, and an embedded **Rhai scripting engine**.

---

## 🌟 Key Features

* **Cross-Platform**: Runs natively on Linux, macOS, and Windows.
* **Non-Blocking Architecture**: Multi-threaded design using `crossbeam-channel` keeps the UI responsive (60+ FPS) even during high-throughput serial transfers.
* **Display Modes**: Live switching between **ASCII**, **HEX**, and **Mixed** (HEX + ASCII side-by-side) terminal output views.
* **Flexible EOL (Line Endings)**: Support for `None`, `NULL (0x00)`, `CR (\r)`, `LF (\n)`, `CRLF (\r\n)`, and `ETX (0x03)`.
* **Command History**:
* Step through sent commands using `Up` / `Down` arrow keys inside the command input box.
* Instant drop-down selector (`📜 History`) to quickly pick previously executed commands.


* **Preconfigured Command Macros (Presets)**:
* Store, manage, and label custom command sequences (e.g., `AT`, `ATI`, `AT+RST`).
* One-click **Send**, **Load into input**, or **Delete** macros.


* **Paced Binary File Transfer**:
* Send firmware/binary payloads via native file dialog (`rfd`).
* Configurable **chunk size** and **inter-chunk delay (ms)** to prevent receiver buffer overruns.
* Real-time transfer progress bar.


* **Embedded Rhai Scripting Engine**:
* Execute automated testing and tactical rpg scripts in a background thread.
* Built-in syntax-highlighted code editor.
* Exposed API functions: `serial_send(data)`, `serial_read_line(timeout_ms)`, and `print(msg)`.


* **System Console Log**: Separate diagnostic window recording timestamped connection events, transfers, script logs, and hardware errors.
* **Theme Switcher**: Instant toggle between **Dark** and **Light** themes.

---

## 🏗 System Architecture

```
┌────────────────────────────────────────────────────────┐
│                   egui Frontend UI                     │
│       (Main Thread - 60+ FPS Rendering Engine)         │
└──────────────┬──────────────────────────▲──────────────┘
               │ Commands                 │ Rx Bytes /
               │ (TxCmd)                  │ RxEvents
               ▼                          │
┌─────────────────────────────────────────┴──────────────┐
│                  Serial Worker Thread                  │
│       (Dedicated I/O Thread via serialport-rs)         │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
                  Physical Serial Device
                (USB-UART / RS-232 / TTL)

```

---

## 📋 Prerequisites

### Linux Dependencies

`serialport-rs` requires `pkg-config` and `libudev` header files on Linux systems:

* **Ubuntu / Debian**:
```bash
sudo apt update
sudo apt install pkg-config libudev-dev

```


* **Arch Linux**:
```bash
sudo pacman -S pkgconf systemd

```


* **Fedora**:
```bash
sudo dnf install pkgconf-pkg-config systemd-devel

```

* **Gentoo** Ebuild (serialforge-0.1.0.ebuild)
source is und packageing/serialforge.ebuild
Place in your custom overlay at dev-embedded/serialforge/serialforge-0.1.0.ebuild:


---

## 🚀 Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/your-username/serialforge.git
cd serialforge

```

### 2. Build and Run

```bash
cargo run --release

```

---

## 📜 Rhai Scripting API

`serialforge` integrates the **Rhai** scripting engine, allowing you to write automated test sequences directly inside the application.

### Available Functions

| Function | Return Type | Description |
| --- | --- | --- |
| `serial_send("text")` | `()` | Sends the string payload to the connected serial port. |
| `serial_read_line(timeout_ms)` | `String` | Blocks up to `timeout_ms` waiting for a line ending in `\r` or `\n`. |
| `print("message")` | `()` | Prints output to the Script Execution Log panel. |

### Example Script

```rhai
// Simple AT Command Handshake Script
print("Starting device test...");

// Send AT Command
serial_send("AT\r\n");

// Read response within 2000 ms
let response = serial_read_line(2000);
print("Received: " + response);

if response.contains("OK") {
    print("Handshake successful! Querying device info...");
    serial_send("ATI\r\n");
    let info = serial_read_line(2000);
    print("Device Info: " + info);
} else {
    print("Error: Device did not respond with OK.");
}

```

---

## License & AI Provenance

This project is licensed under the [MIT License](LICENSE).

This repository contains AI-assisted code generation. For full disclosures on human vs. AI authorship, copyright status, and third-party dependency tracking, see [`PROVENANCE.md`](PROVENANCE.md) and [`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md).
