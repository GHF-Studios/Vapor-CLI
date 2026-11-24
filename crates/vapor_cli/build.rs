use std::env;

fn main() {
    match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("linux") => println!("cargo:rustc-link-arg-bin=vapor=-Wl,-rpath,$ORIGIN"),
        Ok("macos") => println!("cargo:rustc-link-arg-bin=vapor=-Wl,-rpath,@loader_path"),
        _ => {}
    }
}
