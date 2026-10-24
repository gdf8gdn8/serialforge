# serialforge TODO & Feature Roadmap

This document outlines planned improvements, feature suggestions, and architectural milestones for **serialforge**.

---

## 🎯 Upcoming Feature Roadmap

### 1. Terminal Capture & Session Logging
- [x] **Log-to-File (Stream Capture)**: Live streaming of raw bytes or formatted text to a user-selected timestamped file (e.g. `session_YYYY-MM-DD_HH-MM.log`).
- [x] **Export Console Buffer**: Button to quickly dump the currently buffered terminal view (ASCII, HEX, or Mixed) to disk.

### 2. Connection Resilience & Ergonomics
- [x] **USB Hotplug & Auto-Reconnect**: Detect temporary device resets/disconnects (e.g., during firmware flashing) and automatically re-open the serial port after a configurable interval.
- [x] **Custom Baud Rate Input**: Allow manual entry of arbitrary/non-standard baud rates in addition to the dropdown presets.

### 3. Data Transmission Enhancements
- [x] **HEX / Binary Send Mode**: Option in the Transmit bar to send raw byte sequences (e.g. `01 03 00 00 00 0A C5 CD` or Modbus RTU frames) directly without UTF-8 encoding.
- [x] **Repeat / Periodic Transmit**: Interval timer to repeatedly send a preset or string every $N$ milliseconds.

### 4. Terminal Display & Timestamps
- [ ] **Line-by-Line Timestamps**: Toggle timestamp prefixes per received line or packet (e.g. `[11:42:05.123] RX: <data>`) to aid protocol latency and jitter analysis.
- [ ] **ANSI / VT100 Escape Sequence Support**: Basic color/formatting decoding for embedded serial console output.

### 5. Extended Automation & Scripting (Rhai)
- [x] **Binary & Hex Helpers in Rhai**: Built-in methods like `serial_write_hex("01 03 ...")` and raw byte slice helpers.
- [x] **Pattern Wait Helper**: `serial_wait_for(pattern, timeout_ms)` to simplify expectation assertions.
- [x] **Report Export**: Allow scripts to write results or metrics directly to CSV/JSON test logs.

### 6. Real-time Telemetry & Plotting
- [ ] **egui_plot Integration**: Optional telemetry plotting tab to graph real-time values from CSV or key-value streams (e.g., sensor readings, ADC channels).
