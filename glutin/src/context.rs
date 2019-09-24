use super::*;
use std::ffi::c_void;

#[derive(Debug)]
pub struct Context {
    pub(crate) context: platform_impl::Context,
}

impl Context {
    #[inline]
    pub(crate) fn inner(&self) -> &platform_impl::Context {
        &self.context
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), ContextError> {
        self.context.make_current_surfaceless()
    }

    #[inline]
    pub unsafe fn make_current_surface(
        &self,
        surface: &WindowSurface,
    ) -> Result<(), ContextError> {
        self.context.make_current_surface(surface.inner())
    }

    #[inline]
    pub unsafe fn make_current_pbuffer(
        &self,
        pbuffer: &PBuffer,
    ) -> Result<(), ContextError> {
        self.context.make_current_pbuffer(pbuffer.inner())
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        self.context.make_not_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.context.is_current()
    }

    #[inline]
    pub fn get_surface_config(&self) -> SurfaceConfig {
        self.context.get_surface_config()
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.context.get_api()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const c_void {
        self.context.get_proc_address(addr)
    }

    #[inline]
    pub fn update_after_resize(&self) {
        #[cfg(target_os = "macos")]
        self.context.update_after_resize()
    }
}

impl<'a> ContextBuilder<'a> {
    #[inline]
    pub fn build<TE>(
        self,
        el: &Display,
        supports_surfaceless: bool,
        surface_config: &SurfaceConfig,
    ) -> Result<Context, CreationError> {
        let cb = self.map_sharing(|ctx| &ctx.context);
        platform_impl::Context::new(el, cb, supports_surfaceless, surface_config.with_config(&surface_config.config))
            .map(|context| Context { context })
    }
}
