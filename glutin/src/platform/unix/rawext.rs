/// FIXME: rework

/// A unix-specific extension for the [`ContextBuilder`] which allows
/// assembling [`RawContext<T>`]s.
///
/// [`RawContext<T>`]: ../../type.RawContext.html
/// [`ContextBuilder`]: ../../struct.ContextBuilder.html
pub trait RawContextExt {
    /// Creates a raw context on the provided surface.
    ///
    /// Unsafe behaviour might happen if you:
    ///   - Provide us with invalid parameters.
    ///   - The surface/display_ptr is destroyed before the context
    unsafe fn build_raw_wayland_context(
        self,
        display_ptr: *const wayland::wl_display,
        surface: *mut raw::c_void,
        width: u32,
        height: u32,
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
    where
        Self: Sized;

    /// Creates a raw context on the provided window.
    ///
    /// Unsafe behaviour might happen if you:
    ///   - Provide us with invalid parameters.
    ///   - The xwin is destroyed before the context
    unsafe fn build_raw_x11_context(
        self,
        xconn: Arc<XConnection>,
        xwin: raw::c_ulong,
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
    where
        Self: Sized;
}

impl<'a, CS: ContextCurrentState, PBS: SupportsPBuffersTrait, WST: SupportsWindowSurfacesTrait, ST: SupportsSurfacelessTrait> RawContextExt for ContextBuilder<'a, CS, PBS, WST, ST> {
{
    #[inline]
    unsafe fn build_raw_wayland_context(
        self,
        display_ptr: *const wayland::wl_display,
        surface: *mut raw::c_void,
        width: u32,
        height: u32,
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
    where
        Self: Sized,
    {
        let crate::ContextBuilder { pf_reqs, gl_attr, plat_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;
        let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
            Context::Wayland(ref ctx) => ctx,
            _ => unreachable!(),
        });
        wayland::Context::new_raw_context(
            display_ptr,
            surface,
            width,
            height,
            &pf_reqs,
            &gl_attr,
            &plat_attr,
        )
        .map(|context| Context::Wayland(context))
        .map(|context| crate::Context {
            context,
            phantom: PhantomData,
        })
        .map(|context| crate::RawContext {
            context,
            window: (),
        })
    }

    #[inline]
    unsafe fn build_raw_x11_context(
        self,
        xconn: Arc<XConnection>,
        xwin: raw::c_ulong,
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
    where
        Self: Sized,
    {
        let crate::ContextBuilder { pf_reqs, gl_attr, plat_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
        let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
            Context::X11(ref ctx) => ctx,
            _ => unreachable!(),
        });
        x11::Context::new_raw_context(xconn, xwin, &pf_reqs, &gl_attr, &plat_attr)
            .map(|context| Context::X11(context))
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
