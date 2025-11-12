fn main() {
    #[cfg(windows)]
    {
        extern crate windres;
        windres::Build::new().compile("kashikishi-icon.rc").unwrap();
    }
}
