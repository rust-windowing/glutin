//! Parts of AppKit related to OpenGL that is not yet in `objc2-app-kit`.
#![allow(dead_code)]
#![allow(non_snake_case)]

use objc2::encode::{Encoding, RefEncode};
use objc2::rc::{Allocated, Retained};
use objc2::{extern_class, extern_methods, AllocAnyThread, MainThreadMarker};
#[allow(deprecated)]
use objc2_app_kit::{NSOpenGLContextParameter, NSOpenGLPixelFormatAttribute, NSView};
use objc2_foundation::NSObject;

pub type GLint = i32;

#[repr(C)]
pub struct CGLContextObj {
    __inner: [u8; 0],
}

unsafe impl RefEncode for CGLContextObj {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("_CGLContextObject", &[]));
}

extern_class!(
    #[unsafe(super(NSObject))]
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct NSOpenGLContext;
);

unsafe impl Send for NSOpenGLContext {}
unsafe impl Sync for NSOpenGLContext {}

impl NSOpenGLContext {
    extern_methods!(
        #[unsafe(method(currentContext))]
        pub(crate) fn currentContext() -> Option<Retained<Self>>;

        #[unsafe(method(initWithFormat:shareContext:))]
        pub(crate) fn initWithFormat_shareContext(
            this: Allocated<Self>,
            format: &NSOpenGLPixelFormat,
            share: Option<&NSOpenGLContext>,
        ) -> Option<Retained<Self>>;

        #[unsafe(method(clearCurrentContext))]
        pub(crate) fn clearCurrentContext();

        #[unsafe(method(makeCurrentContext))]
        pub(crate) fn makeCurrentContext(&self);

        #[unsafe(method(update))]
        pub(crate) fn update(&self);

        #[unsafe(method(flushBuffer))]
        pub(crate) fn flushBuffer(&self);

        #[unsafe(method(view))]
        pub(crate) fn view(&self, mtm: MainThreadMarker) -> Option<Retained<NSView>>;

        #[unsafe(method(setView:))]
        pub(crate) unsafe fn setView(&self, view: Option<&NSView>);

        #[allow(deprecated)]
        #[unsafe(method(setValues:forParameter:))]
        pub(crate) unsafe fn setValues_forParameter(
            &self,
            vals: *const GLint,
            param: NSOpenGLContextParameter,
        );

        #[unsafe(method(CGLContextObj))]
        pub(crate) fn CGLContextObj(&self) -> *mut CGLContextObj;
    );
}

extern_class!(
    #[unsafe(super(NSObject))]
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct NSOpenGLPixelFormat;
);

unsafe impl Send for NSOpenGLPixelFormat {}
unsafe impl Sync for NSOpenGLPixelFormat {}

impl NSOpenGLPixelFormat {
    extern_methods!(
        #[unsafe(method(initWithAttributes:))]
        unsafe fn initWithAttributes(
            this: Allocated<Self>,
            attrs: *const NSOpenGLPixelFormatAttribute,
        ) -> Option<Retained<Self>>;

        #[unsafe(method(getValues:forAttribute:forVirtualScreen:))]
        pub(crate) unsafe fn getValues_forAttribute_forVirtualScreen(
            &self,
            vals: *mut GLint,
            param: NSOpenGLPixelFormatAttribute,
            screen: GLint,
        );
    );

    pub(crate) unsafe fn newWithAttributes(
        attrs: &[NSOpenGLPixelFormatAttribute],
    ) -> Option<Retained<Self>> {
        unsafe { Self::initWithAttributes(Self::alloc(), attrs.as_ptr()) }
    }
}
