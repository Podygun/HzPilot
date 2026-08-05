fn main() {
    println!("cargo:rerun-if-changed=assets/hzpilot.ico");
    println!("cargo:rerun-if-changed=assets/app.manifest");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let version = std::env::var("CARGO_PKG_VERSION").expect("package version is unavailable");
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("assets/hzpilot.ico")
            .set_manifest_file("assets/app.manifest")
            .set("FileDescription", "HzPilot refresh rate switcher")
            .set("ProductName", "HzPilot")
            .set("ProductVersion", &version)
            .set("FileVersion", &version)
            .set("OriginalFilename", "hzpilot.exe")
            .set("LegalCopyright", "Copyright (c) 2026 HzPilot contributors");
        resource
            .compile()
            .expect("failed to compile Windows resources");
    }
}
