use gl_generator::{Api, Fallbacks, Profile, Registry};
use std::env;
use std::fs::File;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap();
    let dest = PathBuf::from(&env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=build.rs");

    if target.contains("ios") {
        println!("cargo:rustc-link-lib=framework=GLKit");
        println!("cargo:rustc-link-lib=framework=OpenGLES");
        let mut file = File::create(dest.join("gles2_bindings.rs")).unwrap();
        Registry::new(Api::Gles2, (2, 0), Profile::Core, Fallbacks::None, [])
            .write_bindings(gl_generator::StaticStructGenerator, &mut file)
            .unwrap();
    }
}
