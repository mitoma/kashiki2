fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("kashikishi-icon.ico");
        res.compile().unwrap();
    }
}
