fn main() {
    // Only compile resources when target OS is Windows
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icon_path = std::path::Path::new("assets/serialforge.ico");
        if icon_path.exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon(icon_path.to_str().unwrap());

            // Set MinGW cross-compiler binaries when cross-compiling from a non-Windows host
            let is_cross = cfg!(not(target_os = "windows"));
            if is_cross && std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default() == "gnu" {
                res.set_toolkit_path("");
                res.set_ar_path("x86_64-w64-mingw32-ar");
                res.set_windres_path("x86_64-w64-mingw32-windres");
            }

            if let Err(e) = res.compile() {
                eprintln!("Warning: Could not compile Windows resource icon: {e}");
            }
        } else {
            println!(
                "cargo:warning=assets/serialforge.ico not found, skipping Windows executable icon."
            );
        }
    }
}
