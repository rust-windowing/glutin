use super::*;

pub trait LighterSurfaceOrNothing {
    type NotInUseType: LighterSurfaceOrNothing;
    type PossiblyInUseType: LighterSurfaceOrNothing;

    unsafe fn treat_as_not_current(self) -> Self::NotInUseType;

    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType;
    unsafe fn make_not_current(
        self,
    ) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)>;
}

impl LighterSurfaceOrNothing for () {
    type NotInUseType = ();
    type PossiblyInUseType = ();

    #[inline]
    unsafe fn treat_as_not_current(self) -> Self::NotInUseType {
        ()
    }

    #[inline]
    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType {
        ()
    }

    #[inline]
    unsafe fn make_not_current(
        self,
    ) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)>
    {
        Ok(())
    }
}
impl<T: LighterSurface> LighterSurfaceOrNothing for T {
    type NotInUseType = <T as LighterSurface>::NotInUseType;
    type PossiblyInUseType = <T as LighterSurface>::PossiblyInUseType;

    #[inline]
    unsafe fn treat_as_not_current(self) -> Self::NotInUseType {
        self.treat_as_not_current()
    }

    #[inline]
    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType {
        self.treat_as_current()
    }

    #[inline]
    unsafe fn make_not_current(
        self,
    ) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)>
    {
        self.make_not_current()
    }
}

#[derive(Debug)]
pub struct UnifiedContext<
    IC: ContextIsCurrentTrait,
    PBT: SupportsPBuffersTrait,
    WST: SupportsWindowSurfacesTrait,
    ST: SupportsSurfacelessTrait,
    SURFACE: LighterSurfaceOrNothing,
> {
    pub(crate) context: SplitContext<IC, PBT, WST, ST>,
    pub(crate) surface: SURFACE,
}

impl<
        IC: ContextIsCurrentTrait,
        PBT: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
        SURFACE: LighterSurfaceOrNothing,
    > UnifiedContext<IC, PBT, WST, ST, SURFACE>
{
    #[inline]
    pub unsafe fn make_not_current(
        self,
    ) -> Result<
        UnifiedContext<
            ContextIsCurrent::No,
            PBT,
            WST,
            ST,
            SURFACE::NotInUseType,
        >,
        (
            UnifiedContext<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                WST,
                ST,
                SURFACE::PossiblyInUseType,
            >,
            ContextError,
        ),
    > {
        match self.context.make_not_current() {
            Ok(context) => match self.surface.make_not_current() {
                Err((surface, err)) => Err((
                    UnifiedContext {
                        context: context.treat_as_current(),
                        surface,
                    },
                    err,
                )),
                Ok(surface) => Ok(UnifiedContext { context, surface }),
            },
            Err((context, err)) => Err((
                UnifiedContext {
                    context,
                    surface: LighterSurfaceOrNothing::treat_as_current(
                        self.surface,
                    ),
                },
                err,
            )),
        }
    }

    #[inline]
    pub unsafe fn split(self) -> (SplitContext<IC, PBT, WST, ST>, SURFACE) {
        (self.context, self.surface)
    }
}

impl<
        IC: ContextIsCurrentTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
        IU: SurfaceInUseTrait,
    > UnifiedContext<IC, SupportsPBuffers::Yes, WST, ST, LighterPBuffer<IU>>
{
    #[inline]
    pub unsafe fn make_current(
        self,
    ) -> Result<
        UnifiedContext<
            ContextIsCurrent::Possibly,
            SupportsPBuffers::Yes,
            WST,
            ST,
            LighterPBuffer<SurfaceInUse::Possibly>,
        >,
        (
            UnifiedContext<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                SupportsPBuffers::Yes,
                WST,
                ST,
                LighterPBuffer<SurfaceInUse::Possibly>,
            >,
            ContextError,
        ),
    > {
        match self.context.make_current_pbuffer(self.surface) {
            Ok((context, surface)) => Ok(UnifiedContext { context, surface }),
            Err((context, surface, err)) => {
                Err((UnifiedContext { context, surface }, err))
            }
        }
    }
}

impl<
        IC: ContextIsCurrentTrait,
        PBT: SupportsPBuffersTrait,
        ST: SupportsSurfacelessTrait,
        IU: SurfaceInUseTrait,
        W,
    >
    UnifiedContext<
        IC,
        PBT,
        SupportsWindowSurfaces::Yes,
        ST,
        LighterWindowSurfaceWrapper<W, IU>,
    >
{
    #[inline]
    pub unsafe fn make_current(
        self,
    ) -> Result<
        UnifiedContext<
            ContextIsCurrent::PossiblyAndSurfaceBound,
            PBT,
            SupportsWindowSurfaces::Yes,
            ST,
            LighterWindowSurfaceWrapper<W, SurfaceInUse::Possibly>,
        >,
        (
            UnifiedContext<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                SupportsWindowSurfaces::Yes,
                ST,
                LighterWindowSurfaceWrapper<W, SurfaceInUse::Possibly>,
            >,
            ContextError,
        ),
    > {
        match self.context.make_current_window(self.surface) {
            Ok((context, surface)) => Ok(UnifiedContext { context, surface }),
            Err((context, surface, err)) => {
                Err((UnifiedContext { context, surface }, err))
            }
        }
    }
}

impl<
        IC: ContextIsCurrentTrait,
        PBT: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
    > UnifiedContext<IC, PBT, WST, SupportsSurfaceless::Yes, ()>
{
    #[inline]
    pub unsafe fn make_current(
        self,
    ) -> Result<
        UnifiedContext<
            ContextIsCurrent::Possibly,
            PBT,
            WST,
            SupportsSurfaceless::Yes,
            (),
        >,
        (
            UnifiedContext<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                WST,
                SupportsSurfaceless::Yes,
                (),
            >,
            ContextError,
        ),
    > {
        match self.context.make_current_surfaceless() {
            Ok(context) => Ok(UnifiedContext {
                context,
                surface: (),
            }),
            Err((context, err)) => Err((
                UnifiedContext {
                    context,
                    surface: (),
                },
                err,
            )),
        }
    }
}

impl<
        IC: ContextIsCurrentTrait,
        PBT: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
        SURFACE: LighterSurfaceOrNothing,
    > UnifiedContext<IC, PBT, WST, SupportsSurfaceless::Yes, SURFACE>
{
    #[inline]
    pub unsafe fn make_current_surfaceless(
        self,
    ) -> Result<
        UnifiedContext<
            ContextIsCurrent::Possibly,
            PBT,
            WST,
            SupportsSurfaceless::Yes,
            SURFACE::NotInUseType,
        >,
        (
            UnifiedContext<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                WST,
                SupportsSurfaceless::Yes,
                SURFACE,
            >,
            ContextError,
        ),
    > {
        match self.context.make_current_surfaceless() {
            Ok(context) => Ok(UnifiedContext {
                context,
                surface: self.surface.treat_as_not_current(),
            }),
            Err((context, err)) => Err((
                UnifiedContext {
                    context,
                    surface: self.surface,
                },
                err,
            )),
        }
    }
}

impl<
        IC: ContextIsCurrentTrait,
        PBT: SupportsPBuffersTrait,
        ST: SupportsSurfacelessTrait,
        SURFACE: LighterSurfaceOrNothing,
    > UnifiedContext<IC, PBT, SupportsWindowSurfaces::Yes, ST, SURFACE>
{
    #[inline]
    pub unsafe fn make_current_window<W, IU: SurfaceInUseTrait>(
        self,
        surface: LighterWindowSurfaceWrapper<W, IU>,
    ) -> Result<
        (
            UnifiedContext<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                SupportsWindowSurfaces::Yes,
                ST,
                LighterWindowSurfaceWrapper<W, SurfaceInUse::Possibly>,
            >,
            SURFACE::NotInUseType,
        ),
        (
            UnifiedContext<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                SupportsWindowSurfaces::Yes,
                ST,
                SURFACE,
            >,
            LighterWindowSurfaceWrapper<W, SurfaceInUse::Possibly>,
            ContextError,
        ),
    > {
        match self.context.make_current_window(surface) {
            Ok((context, nsurface)) => Ok((
                UnifiedContext {
                    context,
                    surface: LighterSurface::treat_as_current(nsurface),
                },
                LighterSurfaceOrNothing::treat_as_not_current(self.surface),
            )),
            Err((context, nsurface, err)) => Err((
                UnifiedContext {
                    context,
                    surface: self.surface,
                },
                LighterSurface::treat_as_current(nsurface),
                err,
            )),
        }
    }
}

impl<
        IC: ContextIsCurrentTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
        SURFACE: LighterSurfaceOrNothing,
    > UnifiedContext<IC, SupportsPBuffers::Yes, WST, ST, SURFACE>
{
    #[inline]
    pub unsafe fn make_current_pbuffer<IU: SurfaceInUseTrait>(
        self,
        pbuffer: LighterPBuffer<IU>,
    ) -> Result<
        (
            UnifiedContext<
                ContextIsCurrent::Possibly,
                SupportsPBuffers::Yes,
                WST,
                ST,
                LighterPBuffer<SurfaceInUse::Possibly>,
            >,
            SURFACE::NotInUseType,
        ),
        (
            UnifiedContext<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                SupportsPBuffers::Yes,
                WST,
                ST,
                SURFACE,
            >,
            LighterPBuffer<SurfaceInUse::Possibly>,
            ContextError,
        ),
    > {
        match self.context.make_current_pbuffer(pbuffer) {
            Ok((context, pbuffer)) => Ok((
                UnifiedContext {
                    context,
                    surface: LighterSurface::treat_as_current(pbuffer),
                },
                LighterSurfaceOrNothing::treat_as_not_current(self.surface),
            )),
            Err((context, pbuffer, err)) => Err((
                UnifiedContext {
                    context,
                    surface: self.surface,
                },
                LighterSurface::treat_as_current(pbuffer),
                err,
            )),
        }
    }
}
