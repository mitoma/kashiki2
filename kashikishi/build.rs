// build.rs
extern crate windres;

#[cfg(windows)]
fn main() {
    windres::Build::new().compile("kashikishi-icon.rc").unwrap();
}
