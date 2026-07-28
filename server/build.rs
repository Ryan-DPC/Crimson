extern crate winres;
use std::env;
use std::path::PathBuf;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        
        // Find the project root icon path
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        
        // Path: server -> crimson/src-tauri/icons/icon.ico
        let icon_path = manifest_dir.join("..").join("crimson").join("src-tauri").join("icons").join("icon.ico");

        if icon_path.exists() {
            res.set_icon(icon_path.to_str().unwrap());
        } else {
            println!("cargo:warning=Icon not found at {:?}", icon_path);
        }

        // U+2019 and not a plain ASCII apostrophe: winres escapes `'` as `\'` when it
        // writes resource.rc, but RC has no such escape, so the backslash would end up
        // verbatim in the binary and Task Manager would show `Server\'s`.
        res.set("FileDescription", "Server\u{2019}s");
        res.set("ProductName", "Server\u{2019}s");
        res.set("OriginalFilename", "crimson-server.exe");
        res.compile().unwrap();
    }
}
