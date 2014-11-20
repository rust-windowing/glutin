use std::io::fs;
use std::os;

fn main() {
    let out_dir = Path::new(os::getenv("OUT_DIR").unwrap());
    let my_dir = Path::new(os::getenv("CARGO_MANIFEST_DIR").unwrap())
        .join("misc").join("win32-libs");

    match os::getenv("TARGET").unwrap().as_slice() {
        "i686-pc-windows-gnu" => {
            fs::copy(&my_dir.join("libgdi32-32.a"), &out_dir.join("libgdi32.a")).unwrap();
            fs::copy(&my_dir.join("libopengl32-32.a"), &out_dir.join("libopengl32.a")).unwrap();
            println!("cargo:rustc-flags=-L {} -l gdi32:static -l opengl32:static",
                out_dir.as_str().unwrap());
        },
        "x86_64-pc-windows-gnu" => {
            fs::copy(&my_dir.join("libgdi32-64.a"), &out_dir.join("libgdi32.a")).unwrap();
            fs::copy(&my_dir.join("libopengl32-64.a"), &out_dir.join("libopengl32.a")).unwrap();
            println!("cargo:rustc-flags=-L {} -l gdi32:static -l opengl32:static",
                out_dir.as_str().unwrap());
        },
        _ => ()
    }
}
