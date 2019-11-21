/// FIXME: rework

pub trait RawContextExt {
    /// Creates a raw context on the provided window.
    ///
    /// Unsafe behaviour might happen if you:
    ///   - Provide us with invalid parameters.
    ///   - The window is destroyed before the context
    unsafe fn build_raw_context(
        self,
        hwnd: *mut raw::c_void,
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
    where
        Self: Sized;
}

impl<'a, T: ContextCurrentState> RawContextExt
    for crate::ContextBuilder<'a, T>
{
    #[inline]
    unsafe fn build_raw_context(
        self,
        hwnd: *mut raw::c_void,
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
    where
        Self: Sized,
    {
        let crate::ContextBuilder { pf_reqs, gl_attr, plat_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        Context::new_raw_context(hwnd as *mut _, &pf_reqs, &gl_attr, &plat_attr)
            .map(|context| crate::Context {
                context,
                phantom: PhantomData,
            })
            .map(|context| crate::RawContext {
                context,
                window: (),
            })
    }
}
