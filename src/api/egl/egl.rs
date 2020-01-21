#[cfg(not(target_os = "android"))]
mod egl {
    use crate::api::dlloader::{SymTrait, SymWrapper};
    use crate::api::egl::ffi;

    use libloading;
    use parking_lot::Mutex;
    use winit_types::error::Error;
    use winit_types::platform::OsError;

    use std::os::raw;
    use std::sync::Arc;

    #[cfg(unix)]
    use libloading::os::unix as libloading_os;
    #[cfg(windows)]
    use libloading::os::windows as libloading_os;

    #[derive(Clone)]
    pub struct Egl(pub SymWrapper<ffi::egl::Egl>);

    /// Because `*const raw::c_void` doesn't implement `Sync`.
    unsafe impl Sync for Egl {}

    type EglGetProcAddressType =
        libloading_os::Symbol<unsafe extern "C" fn(*const raw::c_void) -> *const raw::c_void>;

    lazy_static! {
        static ref EGL_GET_PROC_ADDRESS: Arc<Mutex<Option<EglGetProcAddressType>>> =
            Arc::new(Mutex::new(None));
    }

    impl SymTrait for ffi::egl::Egl {
        #[inline]
        fn load_with(lib: &libloading::Library) -> Self {
            let f = move |s: &'static str| -> *const raw::c_void {
                // Check if the symbol is available in the library directly. If
                // it is, just return it.
                match unsafe {
                    lib.get(
                        std::ffi::CString::new(s.as_bytes())
                            .unwrap()
                            .as_bytes_with_nul(),
                    )
                } {
                    Ok(sym) => return *sym,
                    Err(_) => (),
                };

                let mut egl_get_proc_address = (*EGL_GET_PROC_ADDRESS).lock();
                if egl_get_proc_address.is_none() {
                    unsafe {
                        let sym: libloading::Symbol<
                            unsafe extern "C" fn(*const raw::c_void) -> *const raw::c_void,
                        > = lib.get(b"eglGetProcAddress").unwrap();
                        *egl_get_proc_address = Some(sym.into_raw());
                    }
                }

                // The symbol was not available in the library, so ask
                // eglGetProcAddress for it. Note that eglGetProcAddress was
                // only able to look up extension functions prior to EGL 1.5,
                // hence this two-part dance.
                unsafe {
                    (egl_get_proc_address.as_ref().unwrap())(
                        std::ffi::CString::new(s.as_bytes())
                            .unwrap()
                            .as_bytes_with_nul()
                            .as_ptr() as *const raw::c_void,
                    )
                }
            };

            Self::load_with(f)
        }
    }

    impl Egl {
        #[inline]
        pub fn new() -> Result<Self, Error> {
            #[cfg(target_os = "windows")]
            let paths = vec!["libEGL.dll", "atioglxx.dll"];

            #[cfg(not(target_os = "windows"))]
            let paths = vec!["libEGL.so.1", "libEGL.so"];

            SymWrapper::new(paths)
                .map(|i| Egl(i))
                .map_err(|_| make_oserror!(OsError::Misc("Could not load EGL symbols".to_string())))
        }
    }
}

#[cfg(target_os = "android")]
mod egl {
    use crate::api::egl::ffi;

    use winit_types::error::Error;

    #[derive(Clone)]
    pub struct Egl(pub ffi::egl::Egl);

    impl Egl {
        #[inline]
        pub fn new() -> Result<Self, Error> {
            Ok(Egl(ffi::egl::Egl))
        }
    }
}

use std::ops::{Deref, DerefMut};

impl Deref for Egl {
    type Target = super::ffi::egl::Egl;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Egl {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub use egl::*;
