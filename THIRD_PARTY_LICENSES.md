# Third-Party Licenses

## 1. Project License
Primary codebase licensed under the [MIT License](LICENSE).

## 2. External Tools & Unlicensed Dependencies

### `serialforge`
* **Status:** Unlicensed / License Unclear (`LicenseRef-Unclear-SerialForge`)
* **Usage:** External runtime utility invoked via CLI (`std::process::Command`).
* **Policy:** `serialforge` is **NOT** vendored or bundled into this repository's release binaries. Users must install or provide `serialforge` independently.

## 3. Third-Party Crates
All standard Rust dependencies (e.g., `serialport`) are compiled under their respective open-source licenses (MIT / Apache-2.0).