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
        let mut file = File::create(dest.join("egl_bindings.rs")).unwrap();
        let reg = Registry::new(Api::Egl, (1, 5), Profile::Core, Fallbacks::All, [
            "EGL_ANDROID_native_fence_sync",
            "EGL_EXT_buffer_age",
            "EGL_EXT_create_context_robustness",
            "EGL_EXT_device_base",
            "EGL_EXT_device_drm",
            "EGL_EXT_device_drm_render_node",
            "EGL_EXT_device_enumeration",
            "EGL_EXT_device_query",
            "EGL_EXT_device_query_name",
            "EGL_EXT_pixel_format_float",
            "EGL_EXT_platform_base",
            "EGL_EXT_platform_device",
            "EGL_EXT_platform_wayland",
            "EGL_EXT_platform_x11",
            "EGL_EXT_swap_buffers_with_damage",
            "EGL_KHR_create_context",
            "EGL_KHR_create_context_no_error",
            "EGL_KHR_fence_sync",
            "EGL_KHR_platform_android",
            "EGL_KHR_platform_gbm",
            "EGL_KHR_platform_wayland",
            "EGL_KHR_platform_x11",
            "EGL_KHR_swap_buffers_with_damage",
            "EGL_KHR_wait_sync",
            "EGL_MESA_platform_gbm",
        ]);

        if target.contains("ios") {
            reg.write_bindings(gl_generator::StaticStructGenerator, &mut file)
        } else {
            reg.write_bindings(gl_generator::StructGenerator, &mut file)
        }
        .unwrap()
    }
}
