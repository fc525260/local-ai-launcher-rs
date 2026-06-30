fn main() {
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/local-ai-launcher.ico");
        resource
            .compile()
            .expect("failed to compile Windows resources");
    }
}
