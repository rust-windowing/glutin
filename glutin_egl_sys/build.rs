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
        || target.contains("windows")
        || target.contains("android")
        || target.contains("ios")
    {
        let mut file = File::create(&dest.join("egl_bindings.rs")).unwrap();
        let reg = Registry::new(
            Api::Egl,
            (1, 5),
            Profile::Core,
            Fallbacks::All,
            [
                "EGL_KHR_create_context",
                "EGL_EXT_create_context_robustness",
                "EGL_KHR_create_context_no_error",
                "EGL_KHR_platform_x11",
                "EGL_KHR_platform_android",
                "EGL_KHR_platform_wayland",
                "EGL_KHR_platform_gbm",
                "EGL_EXT_platform_base",
                "EGL_EXT_platform_x11",
                "EGL_MESA_platform_gbm",
                "EGL_EXT_platform_wayland",
                "EGL_EXT_platform_device",
                "EGL_KHR_swap_buffers_with_damage",
                "EGL_KHR_gl_colorspace",
            ],
        );

        if target.contains("android") || target.contains("ios") {
            reg.write_bindings(gl_generator::StaticStructGenerator, &mut file)
        } else {
            reg.write_bindings(gl_generator::StructGenerator, &mut file)
        }
        .unwrap()
    }
}
