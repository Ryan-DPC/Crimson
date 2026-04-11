extern crate winres;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../icons/icon.ico");
        res.set("FileDescription", "Crimson Phantom Server");
        res.set("ProductName", "Crimson");
        res.set("OriginalFilename", "crimson-server.exe");
        res.compile().unwrap();
    }
}
