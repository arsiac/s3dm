#[cfg(windows)]
fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("crates/s3dm-gui/icons/app/icon.ico");
    res.compile().expect("failed to embed Windows resources");
}

#[cfg(not(windows))]
fn main() {}
