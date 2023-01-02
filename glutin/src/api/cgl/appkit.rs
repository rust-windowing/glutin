//! The parts of AppKit related to OpenGL.
//!
//! TODO: Move this to another crate.
#![allow(dead_code)]
#![allow(non_snake_case)]

use std::ops::Deref;

use dispatch::Queue;
use objc2::encode::{Encoding, RefEncode};
use objc2::foundation::{is_main_thread, NSInteger, NSObject};
use objc2::rc::{Id, Shared};
use objc2::{extern_class, extern_methods, msg_send_id, ClassType};

pub type GLint = i32;

pub enum CGLContextObj {}

// XXX borrowed from winit.

// Unsafe wrapper type that allows us to dispatch things that aren't Send.
// This should *only* be used to dispatch to the main queue.
// While it is indeed not guaranteed that these types can safely be sent to
// other threads, we know that they're safe to use on the main thread.
pub(crate) struct MainThreadSafe<T>(pub(crate) T);

unsafe impl<T> Send for MainThreadSafe<T> {}

impl<T> Deref for MainThreadSafe<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// Run closure on the main thread.
pub(crate) fn run_on_main<R: Send>(f: impl FnOnce() -> R + Send) -> R {
    if is_main_thread() {
        f()
    } else {
        Queue::main().exec_sync(f)
    }
}

unsafe impl RefEncode for CGLContextObj {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("_CGLContextObject", &[]));
}

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct NSOpenGLContext;

    unsafe impl ClassType for NSOpenGLContext {
        type Super = NSObject;
    }
);

unsafe impl Send for NSOpenGLContext {}
unsafe impl Sync for NSOpenGLContext {}

extern_methods!(
    unsafe impl NSOpenGLContext {
        pub(crate) fn currentContext() -> Option<Id<Self, Shared>> {
            unsafe { msg_send_id![Self::class(), currentContext] }
        }

        pub(crate) fn newWithFormat_shareContext(
            format: &NSOpenGLPixelFormat,
            share: Option<&NSOpenGLContext>,
        ) -> Option<Id<Self, Shared>> {
            unsafe {
                msg_send_id![
                    msg_send_id![Self::class(), alloc],
                    initWithFormat: format,
                    shareContext: share,
                ]
            }
        }

        #[sel(clearCurrentContext)]
        pub(crate) fn clearCurrentContext();

        #[sel(makeCurrentContext)]
        pub(crate) fn makeCurrentContext(&self);

        #[sel(update)]
        pub(crate) fn update(&self);

        #[sel(flushBuffer)]
        pub(crate) fn flushBuffer(&self);

        pub(crate) fn view(&self) -> Option<Id<NSObject, Shared>> {
            unsafe { msg_send_id![self, view] }
        }

        #[sel(setView:)]
        pub(crate) unsafe fn setView(&self, view: Option<&NSObject>);

        #[sel(setValues:forParameter:)]
        pub(crate) unsafe fn setValues_forParameter(
            &self,
            vals: *const GLint,
            param: NSOpenGLContextParameter,
        );

        #[sel(CGLContextObj)]
        pub(crate) fn CGLContextObj(&self) -> *mut CGLContextObj;
    }
);

extern_class!(
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub(crate) struct NSOpenGLPixelFormat;

    unsafe impl ClassType for NSOpenGLPixelFormat {
        type Super = NSObject;
    }
);

unsafe impl Send for NSOpenGLPixelFormat {}
unsafe impl Sync for NSOpenGLPixelFormat {}

extern_methods!(
    unsafe impl NSOpenGLPixelFormat {
        pub(crate) unsafe fn newWithAttributes(
            attrs: &[NSOpenGLPixelFormatAttribute],
        ) -> Option<Id<Self, Shared>> {
            unsafe {
                msg_send_id![
                    msg_send_id![Self::class(), alloc],
                    initWithAttributes: attrs.as_ptr(),
                ]
            }
        }

        #[sel(getValues:forAttribute:forVirtualScreen:)]
        pub(crate) unsafe fn getValues_forAttribute_forVirtualScreen(
            &self,
            vals: *mut GLint,
            param: NSOpenGLPixelFormatAttribute,
            screen: GLint,
        );
    }
);

type NSOpenGLContextParameter = NSInteger;
pub(crate) const NSOpenGLCPSwapInterval: NSOpenGLContextParameter = 222;
pub(crate) const NSOpenGLCPSurfaceOrder: NSOpenGLContextParameter = 235;
pub(crate) const NSOpenGLCPSurfaceOpacity: NSOpenGLContextParameter = 236;
pub(crate) const NSOpenGLCPSurfaceBackingSize: NSOpenGLContextParameter = 304;
pub(crate) const NSOpenGLCPReclaimResources: NSOpenGLContextParameter = 308;
pub(crate) const NSOpenGLCPCurrentRendererID: NSOpenGLContextParameter = 309;
pub(crate) const NSOpenGLCPGPUVertexProcessing: NSOpenGLContextParameter = 310;
pub(crate) const NSOpenGLCPGPUFragmentProcessing: NSOpenGLContextParameter = 311;
pub(crate) const NSOpenGLCPHasDrawable: NSOpenGLContextParameter = 314;
pub(crate) const NSOpenGLCPMPSwapsInFlight: NSOpenGLContextParameter = 315;

pub(crate) type NSOpenGLPixelFormatAttribute = u32;
pub(crate) const NSOpenGLPFAAllRenderers: NSOpenGLPixelFormatAttribute = 1;
pub(crate) const NSOpenGLPFATripleBuffer: NSOpenGLPixelFormatAttribute = 3;
pub(crate) const NSOpenGLPFADoubleBuffer: NSOpenGLPixelFormatAttribute = 5;
pub(crate) const NSOpenGLPFAStereo: NSOpenGLPixelFormatAttribute = 6;
pub(crate) const NSOpenGLPFAAuxBuffers: NSOpenGLPixelFormatAttribute = 7;
pub(crate) const NSOpenGLPFAColorSize: NSOpenGLPixelFormatAttribute = 8;
pub(crate) const NSOpenGLPFAAlphaSize: NSOpenGLPixelFormatAttribute = 11;
pub(crate) const NSOpenGLPFADepthSize: NSOpenGLPixelFormatAttribute = 12;
pub(crate) const NSOpenGLPFAStencilSize: NSOpenGLPixelFormatAttribute = 13;
pub(crate) const NSOpenGLPFAAccumSize: NSOpenGLPixelFormatAttribute = 14;
pub(crate) const NSOpenGLPFAMinimumPolicy: NSOpenGLPixelFormatAttribute = 51;
pub(crate) const NSOpenGLPFAMaximumPolicy: NSOpenGLPixelFormatAttribute = 52;
pub(crate) const NSOpenGLPFAOffScreen: NSOpenGLPixelFormatAttribute = 53;
pub(crate) const NSOpenGLPFAFullScreen: NSOpenGLPixelFormatAttribute = 54;
pub(crate) const NSOpenGLPFASampleBuffers: NSOpenGLPixelFormatAttribute = 55;
pub(crate) const NSOpenGLPFASamples: NSOpenGLPixelFormatAttribute = 56;
pub(crate) const NSOpenGLPFAAuxDepthStencil: NSOpenGLPixelFormatAttribute = 57;
pub(crate) const NSOpenGLPFAColorFloat: NSOpenGLPixelFormatAttribute = 58;
pub(crate) const NSOpenGLPFAMultisample: NSOpenGLPixelFormatAttribute = 59;
pub(crate) const NSOpenGLPFASupersample: NSOpenGLPixelFormatAttribute = 60;
pub(crate) const NSOpenGLPFASampleAlpha: NSOpenGLPixelFormatAttribute = 61;
pub(crate) const NSOpenGLPFARendererID: NSOpenGLPixelFormatAttribute = 70;
pub(crate) const NSOpenGLPFASingleRenderer: NSOpenGLPixelFormatAttribute = 71;
pub(crate) const NSOpenGLPFANoRecovery: NSOpenGLPixelFormatAttribute = 72;
pub(crate) const NSOpenGLPFAAccelerated: NSOpenGLPixelFormatAttribute = 73;
pub(crate) const NSOpenGLPFAClosestPolicy: NSOpenGLPixelFormatAttribute = 74;
pub(crate) const NSOpenGLPFARobust: NSOpenGLPixelFormatAttribute = 75;
pub(crate) const NSOpenGLPFABackingStore: NSOpenGLPixelFormatAttribute = 76;
pub(crate) const NSOpenGLPFAMPSafe: NSOpenGLPixelFormatAttribute = 78;
pub(crate) const NSOpenGLPFAWindow: NSOpenGLPixelFormatAttribute = 80;
pub(crate) const NSOpenGLPFAMultiScreen: NSOpenGLPixelFormatAttribute = 81;
pub(crate) const NSOpenGLPFACompliant: NSOpenGLPixelFormatAttribute = 83;
pub(crate) const NSOpenGLPFAScreenMask: NSOpenGLPixelFormatAttribute = 84;
pub(crate) const NSOpenGLPFAPixelBuffer: NSOpenGLPixelFormatAttribute = 90;
pub(crate) const NSOpenGLPFARemotePixelBuffer: NSOpenGLPixelFormatAttribute = 91;
pub(crate) const NSOpenGLPFAAllowOfflineRenderers: NSOpenGLPixelFormatAttribute = 96;
pub(crate) const NSOpenGLPFAAcceleratedCompute: NSOpenGLPixelFormatAttribute = 97;
pub(crate) const NSOpenGLPFAOpenGLProfile: NSOpenGLPixelFormatAttribute = 99;
pub(crate) const NSOpenGLPFAVirtualScreenCount: NSOpenGLPixelFormatAttribute = 128;
// OpenGL Profiles
pub(crate) const NSOpenGLProfileVersionLegacy: NSOpenGLPixelFormatAttribute = 0x1000;
pub(crate) const NSOpenGLProfileVersion3_2Core: NSOpenGLPixelFormatAttribute = 0x3200;
pub(crate) const NSOpenGLProfileVersion4_1Core: NSOpenGLPixelFormatAttribute = 0x4100;
