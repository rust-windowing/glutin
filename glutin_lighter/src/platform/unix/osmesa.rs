pub use glutin::platform::unix::osmesa::*;

use crate::{
    Api, ContextBuilderWrapper, ContextError, ContextIsCurrent,
    ContextIsCurrentTrait, ContextIsCurrentYesTrait, CreationError,
    LighterSurface, PixelFormat, SurfaceInUse, SurfaceInUseTrait,
};
use glutin::dpi;
use std::marker::PhantomData;

/// A unix-specific extension to the [`ContextBuilder`] which allows building
/// unix-specific osmesa contexts.
///
/// [`ContextBuilder`]: ../../struct.ContextBuilder.html
pub trait LighterOsMesaContextExt {
    /// Builds an OsMesa context.
    ///
    /// Errors can occur if the OpenGL [`Context`] could not be created. This
    /// generally happens because the underlying platform doesn't support a
    /// requested feature.
    ///
    /// [`Context`]: struct.Context.html
    fn build_osmesa_lighter(
        self,
    ) -> Result<SplitOsMesaContext<ContextIsCurrent::No>, CreationError>
    where
        Self: Sized;
}

impl<'a, T: ContextIsCurrentTrait> LighterOsMesaContextExt
    for LighterOsMesaContextBuilder<'a, T>
{
    #[inline]
    fn build_osmesa_lighter(
        self,
    ) -> Result<SplitOsMesaContext<ContextIsCurrent::No>, CreationError>
    where
        Self: Sized,
    {
        let cb = self.map_sharing(|ctx| &ctx.context);
        cb.build_osmesa().map(|context| SplitOsMesaContext {
            context,
            phantom: PhantomData,
        })
    }
}

pub type LighterOsMesaContextBuilder<'a, IC> =
    ContextBuilderWrapper<&'a SplitOsMesaContext<IC>>;

#[derive(Debug)]
pub struct SplitOsMesaContext<IC: ContextIsCurrentTrait> {
    pub(crate) context: OsMesaContext,
    pub(crate) phantom: PhantomData<IC>,
}

impl<IC: ContextIsCurrentYesTrait> SplitOsMesaContext<IC> {
    /// Returns the address of an OpenGL function.
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }
}

impl<IC: ContextIsCurrentTrait> SplitOsMesaContext<IC> {
    pub fn is_current(&self) -> bool {
        self.context.is_current()
    }

    pub fn get_api(&self) -> Api {
        self.context.get_api()
    }

    pub unsafe fn make_current_osmesa_buffer<IU: SurfaceInUseTrait>(
        self,
        buffer: LighterOsMesaBuffer<IU>,
    ) -> Result<
        (
            SplitOsMesaContext<ContextIsCurrent::Possibly>,
            LighterOsMesaBuffer<SurfaceInUse::Possibly>,
        ),
        (
            SplitOsMesaContext<ContextIsCurrent::Possibly>,
            LighterOsMesaBuffer<SurfaceInUse::Possibly>,
            ContextError,
        ),
    > {
        match self.context.make_current_osmesa_buffer(buffer.inner()) {
            Ok(()) => Ok((self.treat_as_current(), buffer.treat_as_current())),
            Err(err) => {
                Err((self.treat_as_current(), buffer.treat_as_current(), err))
            }
        }
    }

    pub unsafe fn make_not_current(
        self,
    ) -> Result<SplitOsMesaContext<ContextIsCurrent::No>, (Self, ContextError)>
    {
        match self.context.make_not_current() {
            Ok(()) => Ok(SplitOsMesaContext {
                context: self.context,
                phantom: PhantomData,
            }),
            Err(err) => Err((self, err)),
        }
    }

    pub unsafe fn treat_as_current<IC2: ContextIsCurrentYesTrait>(
        self,
    ) -> SplitOsMesaContext<IC2> {
        SplitOsMesaContext {
            context: self.context,
            phantom: PhantomData,
        }
    }

    pub unsafe fn treat_as_not_current(
        self,
    ) -> SplitOsMesaContext<ContextIsCurrent::No> {
        SplitOsMesaContext {
            context: self.context,
            phantom: PhantomData,
        }
    }

    pub(crate) fn inner(&self) -> &OsMesaContext {
        &self.context
    }

    pub fn unify_with<IU: SurfaceInUseTrait>(
        self,
        buffer: LighterOsMesaBuffer<IU>,
    ) -> UnifiedOsMesaContext<IC, IU> {
        UnifiedOsMesaContext {
            context: self,
            buffer,
        }
    }
}

#[derive(Debug)]
pub struct LighterOsMesaBuffer<IU: SurfaceInUseTrait> {
    pub(crate) buffer: OsMesaBuffer,
    pub(crate) phantom: PhantomData<IU>,
}

impl<IU: SurfaceInUseTrait> LighterSurface for LighterOsMesaBuffer<IU> {
    type Inner = OsMesaBuffer;
    type NotInUseType = LighterOsMesaBuffer<SurfaceInUse::No>;
    type PossiblyInUseType = LighterOsMesaBuffer<SurfaceInUse::Possibly>;

    fn inner(&self) -> &Self::Inner {
        &self.buffer
    }

    fn get_pixel_format(&self) -> PixelFormat {
        self.buffer.get_pixel_format()
    }

    fn is_current(&self) -> bool {
        panic!("This cannot be implemented with OsMesa.")
    }

    unsafe fn treat_as_not_current(self) -> Self::NotInUseType {
        LighterOsMesaBuffer {
            buffer: self.buffer,
            phantom: PhantomData,
        }
    }

    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType {
        LighterOsMesaBuffer {
            buffer: self.buffer,
            phantom: PhantomData,
        }
    }

    unsafe fn make_not_current(
        self,
    ) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)>
    {
        panic!("This cannot be implemented with OsMesa.")
    }
}

impl<IU: SurfaceInUseTrait> LighterOsMesaBuffer<IU> {
    pub fn new<IC: ContextIsCurrentTrait>(
        ctx: &SplitOsMesaContext<IC>,
        size: dpi::PhysicalSize,
    ) -> Result<LighterOsMesaBuffer<SurfaceInUse::No>, CreationError> {
        let ctx = ctx.inner();
        OsMesaBuffer::new(ctx, size).map(|buffer| LighterOsMesaBuffer {
            buffer,
            phantom: PhantomData,
        })
    }
}

#[derive(Debug)]
pub struct UnifiedOsMesaContext<
    IC: ContextIsCurrentTrait,
    IU: SurfaceInUseTrait,
> {
    pub(crate) context: SplitOsMesaContext<IC>,
    pub(crate) buffer: LighterOsMesaBuffer<IU>,
}

impl<IC: ContextIsCurrentTrait, IU: SurfaceInUseTrait>
    UnifiedOsMesaContext<IC, IU>
{
    pub unsafe fn make_current(
        self,
    ) -> Result<
        UnifiedOsMesaContext<
            ContextIsCurrent::Possibly,
            SurfaceInUse::Possibly,
        >,
        (
            UnifiedOsMesaContext<
                ContextIsCurrent::Possibly,
                SurfaceInUse::Possibly,
            >,
            ContextError,
        ),
    > {
        match self.context.make_current_osmesa_buffer(self.buffer) {
            Ok((context, buffer)) => {
                Ok(UnifiedOsMesaContext { context, buffer })
            }
            Err((context, buffer, err)) => {
                Err((UnifiedOsMesaContext { context, buffer }, err))
            }
        }
    }
}

impl<IC: ContextIsCurrentTrait, IU: SurfaceInUseTrait>
    UnifiedOsMesaContext<IC, IU>
{
    pub unsafe fn make_current_osmesa_buffer<IU2: SurfaceInUseTrait>(
        self,
        buffer: LighterOsMesaBuffer<IU2>,
    ) -> Result<
        (
            UnifiedOsMesaContext<
                ContextIsCurrent::Possibly,
                SurfaceInUse::Possibly,
            >,
            LighterOsMesaBuffer<SurfaceInUse::No>,
        ),
        (
            UnifiedOsMesaContext<
                ContextIsCurrent::Possibly,
                SurfaceInUse::Possibly,
            >,
            LighterOsMesaBuffer<SurfaceInUse::Possibly>,
            ContextError,
        ),
    > {
        match self.context.make_current_osmesa_buffer(buffer) {
            Ok((context, nbuffer)) => Ok((
                UnifiedOsMesaContext {
                    context,
                    buffer: nbuffer,
                },
                LighterSurface::treat_as_not_current(self.buffer),
            )),
            Err((context, nbuffer, err)) => Err((
                UnifiedOsMesaContext {
                    context,
                    buffer: LighterSurface::treat_as_current(self.buffer),
                },
                nbuffer,
                err,
            )),
        }
    }
}
