//! WGL display initialization and extension loading.

use std::collections::HashSet;
use std::ffi::{self, CStr, OsStr};
use std::fmt;
use std::os::windows::ffi::OsStrExt;
use std::sync::Arc;

use glutin_wgl_sys::wgl;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::System::LibraryLoader as dll_loader;

use crate::config::ConfigTemplate;
use crate::display::{
    AsRawDisplay, DisplayFeatures, DisplayResult, GetDisplayExtensions, RawDisplay,
};
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
use crate::private::Sealed;
use crate::surface::{PbufferSurface, PixmapSurface, SurfaceAttributes, WindowSurface};

use super::config::Config;
use super::context::NotCurrentContext;
use super::surface::Surface;
use super::WglExtra;

/// A WGL display.
#[derive(Debug)]
pub struct Display<D> {
    pub(crate) inner: Arc<DisplayInner<D>>,
}

impl<D> Clone for Display<D> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<D: HasDisplayHandle> AsRef<D> for Display<D> {
    fn as_ref(&self) -> &D {
        self.display()
    }
}

impl<D: HasDisplayHandle> HasDisplayHandle for Display<D> {
    #[inline]
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        self.display().display_handle()
    }
}

impl<D: HasDisplayHandle> Display<D> {
    /// Create WGL display.
    ///
    /// The `native_window` is used to perform extension loading. If it's not
    /// passed the OpenGL will be limited to what it can do, though, basic
    /// operations could still be performed.
    pub fn new<W: HasWindowHandle>(display: D, native_window: Option<W>) -> Result<Self> {
        Self::new_with_display(display, native_window).map_err(Into::into)
    }

    /// Get the underlying display.
    pub fn display(&self) -> &D {
        &self.inner.display
    }

    pub(crate) fn new_with_display<W: HasWindowHandle>(
        display: D,
        native_window: Option<W>,
    ) -> DisplayResult<Self, D> {
        macro_rules! leap {
            ($res:expr) => {{
                match ($res) {
                    Ok(x) => x,
                    Err(e) => {
                        let error = crate::error::Error::from(e);
                        return Err((error, display).into());
                    },
                }
            }};
        }

        match leap!(display.display_handle().and_then(|r| r.display_handle().map(|r| r.as_raw()))) {
            RawDisplayHandle::Windows(..) => {},
            _ => {
                return Err((
                    ErrorKind::NotSupported("provided native display is not supported"),
                    display,
                )
                    .into())
            },
        };

        let name = OsStr::new("opengl32.dll").encode_wide().chain(Some(0)).collect::<Vec<_>>();
        let lib_opengl32 = unsafe { dll_loader::LoadLibraryW(name.as_ptr()) };
        if lib_opengl32 == 0 {
            return Err((ErrorKind::NotFound, display).into());
        }

        // In case native window was provided init extra functions.
        let (wgl_extra, client_extensions) =
            match leap!(native_window.map(|w| w.window_handle().map(|w| w.as_raw())).transpose()) {
                Some(RawWindowHandle::Win32(window)) => unsafe {
                    let (wgl_extra, client_extensions) = match super::load_extra_functions(
                        window.hinstance.map_or(0, |i| i.get()),
                        window.hwnd.get() as _,
                    ) {
                        Ok(x) => x,
                        Err(e) => return Err((e, display).into()),
                    };
                    (Some(wgl_extra), client_extensions)
                },
                _ => (None, HashSet::new()),
            };

        let features = Self::extract_display_features(&client_extensions);

        let inner = Arc::new(DisplayInner {
            lib_opengl32,
            wgl_extra,
            features,
            client_extensions,
            display,
        });

        Ok(Display { inner })
    }

    fn extract_display_features(extensions: &HashSet<&'static str>) -> DisplayFeatures {
        let mut features = DisplayFeatures::empty();

        features.set(
            DisplayFeatures::MULTISAMPLING_PIXEL_FORMATS,
            extensions.contains("WGL_ARB_multisample"),
        );

        features.set(
            DisplayFeatures::FLOAT_PIXEL_FORMAT,
            extensions.contains("WGL_ARB_pixel_format_float"),
        );

        features.set(
            DisplayFeatures::SRGB_FRAMEBUFFERS,
            extensions.contains("WGL_ARB_framebuffer_sRGB")
                || extensions.contains("WGL_EXT_framebuffer_sRGB")
                || extensions.contains("WGL_EXT_colorspace"),
        );

        features.set(
            DisplayFeatures::CREATE_ES_CONTEXT,
            extensions.contains("WGL_EXT_create_context_es2_profile")
                || extensions.contains("WGL_EXT_create_context_es_profile"),
        );

        features.set(DisplayFeatures::SWAP_CONTROL, extensions.contains("WGL_EXT_swap_control"));

        features.set(
            DisplayFeatures::CONTEXT_ROBUSTNESS,
            extensions.contains("WGL_ARB_create_context_robustness"),
        );

        features.set(
            DisplayFeatures::CONTEXT_RELEASE_BEHAVIOR,
            extensions.contains("WGL_ARB_context_flush_control"),
        );

        features.set(
            DisplayFeatures::CONTEXT_NO_ERROR,
            extensions.contains("WGL_ARB_create_context_no_error"),
        );

        features
    }
}

impl<D: HasDisplayHandle> GlDisplay for Display<D> {
    type Config = Config<D>;
    type NotCurrentContext = NotCurrentContext<D>;
    type PbufferSurface = Surface<D, PbufferSurface>;
    type PixmapSurface = Surface<D, PixmapSurface>;
    type WindowSurface<W: HasWindowHandle> = Surface<D, WindowSurface<W>>;

    fn find_configs<W: HasWindowHandle>(
        &self,
        template: ConfigTemplate<W>,
    ) -> Result<Box<dyn Iterator<Item = Self::Config> + '_>> {
        Self::find_configs(self, template)
    }

    fn create_window_surface<W: HasWindowHandle>(
        &self,
        config: &Self::Config,
        surface_attributes: SurfaceAttributes<WindowSurface<W>>,
    ) -> Result<Self::WindowSurface<W>> {
        Self::create_window_surface(self, config, surface_attributes)
    }

    unsafe fn create_pbuffer_surface(
        &self,
        config: &Self::Config,
        surface_attributes: SurfaceAttributes<PbufferSurface>,
    ) -> Result<Self::PbufferSurface> {
        unsafe { Self::create_pbuffer_surface(self, config, surface_attributes) }
    }

    fn create_context<W: HasWindowHandle>(
        &self,
        config: &Self::Config,
        context_attributes: &crate::context::ContextAttributes<W>,
    ) -> Result<Self::NotCurrentContext> {
        Self::create_context(self, config, context_attributes)
    }

    unsafe fn create_pixmap_surface(
        &self,
        config: &Self::Config,
        surface_attributes: SurfaceAttributes<PixmapSurface>,
    ) -> Result<Self::PixmapSurface> {
        unsafe { Self::create_pixmap_surface(self, config, surface_attributes) }
    }

    fn get_proc_address(&self, addr: &CStr) -> *const ffi::c_void {
        unsafe {
            let addr = addr.as_ptr();
            let fn_ptr = wgl::GetProcAddress(addr);
            if !fn_ptr.is_null() {
                fn_ptr.cast()
            } else {
                dll_loader::GetProcAddress(self.inner.lib_opengl32, addr.cast())
                    .map_or(std::ptr::null(), |fn_ptr| fn_ptr as *const _)
            }
        }
    }

    fn version_string(&self) -> String {
        String::from("WGL")
    }

    fn supported_features(&self) -> DisplayFeatures {
        self.inner.features
    }
}

impl<D: HasDisplayHandle> GetDisplayExtensions for Display<D> {
    fn extensions(&self) -> &HashSet<&'static str> {
        &self.inner.client_extensions
    }
}

impl<D: HasDisplayHandle> AsRawDisplay for Display<D> {
    fn raw_display(&self) -> RawDisplay {
        RawDisplay::Wgl
    }
}

impl<D: HasDisplayHandle> Sealed for Display<D> {}

pub(crate) struct DisplayInner<D> {
    /// Client WGL extensions.
    pub(crate) lib_opengl32: HMODULE,

    /// Extra functions used by the impl.
    pub(crate) wgl_extra: Option<&'static WglExtra>,

    pub(crate) features: DisplayFeatures,

    pub(crate) client_extensions: HashSet<&'static str>,

    /// Hold onto the display reference to keep it valid.
    pub(crate) display: D,
}

impl<D> fmt::Debug for DisplayInner<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Display")
            .field("features", &self.features)
            .field("extensions", &self.client_extensions)
            .finish()
    }
}

pub(crate) fn load_extensions(hdc: HDC, wgl_extra: &WglExtra) -> HashSet<&'static str> {
    let extensions = unsafe {
        if wgl_extra.GetExtensionsStringARB.is_loaded() {
            CStr::from_ptr(wgl_extra.GetExtensionsStringARB(hdc as *const _))
        } else if wgl_extra.GetExtensionsStringEXT.is_loaded() {
            CStr::from_ptr(wgl_extra.GetExtensionsStringEXT())
        } else {
            return HashSet::new();
        }
    };

    if let Ok(extensions) = extensions.to_str() {
        extensions.split(' ').collect::<HashSet<_>>()
    } else {
        HashSet::new()
    }
}
