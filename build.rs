fn main() {
    // Only embed icon resources when targeting Windows
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .set_windows_subsystem("windows")
            .compile()
            .expect("Failed to compile Windows resources");
    }
}
