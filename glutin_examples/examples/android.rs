#![cfg(target_os = "android")]

#[ndk_glue::main(backtrace = "on")]
fn main() {
    glutin_examples::main()
}
