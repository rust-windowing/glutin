//! Parts of AppKit related to OpenGL that is not yet in `objc2-app-kit`.
#![allow(dead_code)]
#![allow(non_snake_case)]

use objc2::encode::{Encoding, RefEncode};
use objc2::rc::{Allocated, Id};
use objc2::{extern_class, extern_methods, mutability, ClassType};
#[allow(deprecated)]
use objc2_app_kit::{NSOpenGLContextParameter, NSOpenGLPixelFormatAttribute, NSView};
use objc2_foundation::{MainThreadMarker, NSObject};

pub type GLint = i32;

#[repr(C)]
pub struct CGLContextObj {
    __inner: [u8; 0],
}

unsafe impl RefEncode for CGLContextObj {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("_CGLContextObject", &[]));
}

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct NSOpenGLContext;

    // Strict order required by macro, tracked in https://github.com/madsmtm/objc2/issues/479
    #[rustfmt::skip]
    unsafe impl ClassType for NSOpenGLContext {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

unsafe impl Send for NSOpenGLContext {}
unsafe impl Sync for NSOpenGLContext {}

extern_methods!(
    unsafe impl NSOpenGLContext {
        #[method_id(currentContext)]
        pub(crate) fn currentContext() -> Option<Id<Self>>;

        #[method_id(initWithFormat:shareContext:)]
        pub(crate) fn initWithFormat_shareContext(
            this: Allocated<Self>,
            format: &NSOpenGLPixelFormat,
            share: Option<&NSOpenGLContext>,
        ) -> Option<Id<Self>>;

        #[method(clearCurrentContext)]
        pub(crate) fn clearCurrentContext();

        #[method(makeCurrentContext)]
        pub(crate) fn makeCurrentContext(&self);

        #[method(update)]
        pub(crate) fn update(&self);

        #[method(flushBuffer)]
        pub(crate) fn flushBuffer(&self);

        #[method_id(view)]
        pub(crate) fn view(&self, mtm: MainThreadMarker) -> Option<Id<NSView>>;

        #[method(setView:)]
        pub(crate) unsafe fn setView(&self, view: Option<&NSView>);

        #[allow(deprecated)]
        #[method(setValues:forParameter:)]
        pub(crate) unsafe fn setValues_forParameter(
            &self,
            vals: *const GLint,
            param: NSOpenGLContextParameter,
        );

        #[method(CGLContextObj)]
        pub(crate) fn CGLContextObj(&self) -> *mut CGLContextObj;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct NSOpenGLPixelFormat;

    // Strict order required by macro, tracked in https://github.com/madsmtm/objc2/issues/479
    #[rustfmt::skip]
    unsafe impl ClassType for NSOpenGLPixelFormat {
        type Super = NSObject;
        type Mutability = mutability::Immutable;
    }
);

unsafe impl Send for NSOpenGLPixelFormat {}
unsafe impl Sync for NSOpenGLPixelFormat {}

extern_methods!(
    unsafe impl NSOpenGLPixelFormat {
        #[method_id(initWithAttributes:)]
        unsafe fn initWithAttributes(
            this: Allocated<Self>,
            attrs: *const NSOpenGLPixelFormatAttribute,
        ) -> Option<Id<Self>>;

        pub(crate) unsafe fn newWithAttributes(
            attrs: &[NSOpenGLPixelFormatAttribute],
        ) -> Option<Id<Self>> {
            unsafe { Self::initWithAttributes(Self::alloc(), attrs.as_ptr()) }
        }

        #[method(getValues:forAttribute:forVirtualScreen:)]
        pub(crate) unsafe fn getValues_forAttribute_forVirtualScreen(
            &self,
            vals: *mut GLint,
            param: NSOpenGLPixelFormatAttribute,
            screen: GLint,
        );
    }
);
