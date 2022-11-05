use gl_generator::{Api, Fallbacks, Profile, Registry};
use std::env;
use std::fs::File;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap();
    let dest = PathBuf::from(&env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=build.rs");

    if target.contains("linux")
        || target.contains("dragonfly")
        || target.contains("freebsd")
        || target.contains("netbsd")
        || target.contains("openbsd")
    {
        let mut file = File::create(dest.join("glx_bindings.rs")).unwrap();
        Registry::new(Api::Glx, (1, 4), Profile::Core, Fallbacks::All, [])
            .write_bindings(gl_generator::StructGenerator, &mut file)
            .unwrap();

        let mut file = File::create(dest.join("glx_extra_bindings.rs")).unwrap();
        Registry::new(Api::Glx, (1, 4), Profile::Core, Fallbacks::All, [
            "GLX_ARB_context_flush_control",
            "GLX_ARB_create_context",
            "GLX_ARB_create_context_no_error",
            "GLX_ARB_create_context_profile",
            "GLX_ARB_create_context_robustness",
            "GLX_ARB_fbconfig_float",
            "GLX_ARB_framebuffer_sRGB",
            "GLX_ARB_multisample",
            "GLX_EXT_buffer_age",
            "GLX_EXT_create_context_es2_profile",
            "GLX_EXT_framebuffer_sRGB",
            "GLX_EXT_swap_control",
            "GLX_MESA_swap_control",
            "GLX_SGI_swap_control",
        ])
        .write_bindings(gl_generator::StructGenerator, &mut file)
        .unwrap();
    }
}
