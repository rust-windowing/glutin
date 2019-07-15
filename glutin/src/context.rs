use super::*;
use winit::event_loop::EventLoopWindowTarget;

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
    pub unsafe fn make_current_window(
        &self,
        surface: &WindowSurface,
    ) -> Result<(), ContextError> {
        self.context.make_current_window(surface.inner())
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
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.context.get_pixel_format()
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.context.get_api()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }

    #[inline]
    pub fn update_after_resize(&self) {
        #[cfg(target_os = "macos")]
        self.context.update_after_resize()
    }
}

bitflags! {
    #[derive(Default)]
    pub struct ContextSupports: u8 {
        const PBUFFERS = 1 << 0;
        const WINDOW_SURFACES = 1 << 1;
        const SURFACELESS = 1 << 2;
    }
}

impl<'a> ContextBuilder<'a> {
    #[inline]
    pub fn build<TE>(
        self,
        el: &EventLoopWindowTarget<TE>,
        ctx_supports: ContextSupports,
    ) -> Result<Context, CreationError> {
        let cb = self.map_sharing(|ctx| &ctx.context);
        platform_impl::Context::new(el, cb, ctx_supports)
            .map(|context| Context { context })
    }
}
