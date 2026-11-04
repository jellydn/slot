#[cfg(target_os = "macos")]
#[path = "record/macos.rs"]
mod macos;

#[cfg(target_os = "macos")]
fn main() {
    macos::main();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("record: the headless OpenGL recorder requires macOS");
    std::process::exit(1);
}
