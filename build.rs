fn main() {
    winres::WindowsResource::new()
        .set_icon("tlogo.ico")
        .compile()
        .unwrap();
}
