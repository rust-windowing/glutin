#![cfg(target_os = "macos")]

use CreationError;
use CreationError::OsError;
use ContextError;
use GlAttributes;
use GlContext;
use PixelFormat;
use PixelFormatRequirements;
use Robustness;
use WindowAttributes;
use os::macos::ActivationPolicy;

use objc::runtime::{BOOL, NO};

use cgl::{CGLEnable, kCGLCECrashOnRemovedFunctions, CGLSetParameter, kCGLCPSurfaceOpacity};

use cocoa::base::{id, nil};
use cocoa::foundation::NSAutoreleasePool;
use cocoa::appkit::{self, NSOpenGLContext, NSOpenGLPixelFormat};

use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation::bundle::{CFBundleGetBundleWithIdentifier, CFBundleGetFunctionPointerForName};

use std::cell::RefCell;
use std::collections::HashMap;
use std::str::FromStr;
use std::ops::Deref;
use std::sync::{Arc, Mutex, Weak};

use libc;

use winit::{self, ControlFlow};
use winit::os::macos::WindowExt;
pub use winit::{MonitorId, NativeMonitorId, get_available_monitors, get_primary_monitor};
pub use self::headless::HeadlessContext;
pub use self::headless::PlatformSpecificHeadlessBuilderAttributes;

mod headless;
mod helpers;

#[derive(Clone, Default)]
pub struct PlatformSpecificWindowBuilderAttributes {
    pub activation_policy: ActivationPolicy,
}

pub struct Window {
    // A handle to the GL context associated with this window.
    context: Arc<Context>,
    // The Window must store a handle to the map in order to remove its own context when dropped.
    contexts: Arc<ContextMap>,
    winit_window: winit::Window,
}

pub struct EventsLoop {
    // `winit_events_loop` is wrapped in a refcell to avoid borrowing `self` mutably in `poll_events` or `run_forever`
    // and simultaneously borrowing `self` immutably in `handle_event`.
    winit_events_loop: RefCell<winit::EventsLoop>,
    window_contexts: Mutex<Weak<ContextMap>>,
}

struct Context {
    // NSOpenGLContext
    gl: IdRef,
    pixel_format: PixelFormat,
}

struct ContextMap {
    map: Mutex<HashMap<winit::WindowId, Weak<Context>>>,
}

unsafe impl Send for ContextMap {}
unsafe impl Sync for ContextMap {}

impl EventsLoop {
    /// Builds a new events loop.
    pub fn new() -> EventsLoop {
        EventsLoop {
            winit_events_loop: RefCell::new(winit::EventsLoop::new()),
            window_contexts: Mutex::new(Weak::new()),
        }
    }

    fn handle_event(&self, event: &winit::Event) {
        match *event {
            winit::Event::WindowEvent { window_id, ref event } => match *event {

                // If a `Resized` event was received for a window, update the GL context for that
                // window but only if that window is still alive.
                winit::WindowEvent::Resized(..) => {
                    if let Some(window_contexts) = self.window_contexts.lock().unwrap().upgrade() {
                        if let Some(context) = window_contexts.map.lock().unwrap()[&window_id].upgrade() {
                            unsafe { context.gl.update(); }
                        }
                    }
                },

                // If a `Closed` event was received for a window, remove the associated context
                // from the map.
                winit::WindowEvent::Closed => {
                    if let Some(window_contexts) = self.window_contexts.lock().unwrap().upgrade() {
                        window_contexts.map.lock().unwrap().remove(&window_id);
                    }
                },

                _ => (),
            },
            winit::Event::DeviceEvent { .. } => (), // FIXME: Should this be handled??
            winit::Event::Awakened => (), // FIXME: Should this be handled??
        }
    }

    /// Fetches all the events that are pending, calls the callback function for each of them,
    /// and returns.
    #[inline]
    pub fn poll_events<F>(&mut self, mut callback: F)
        where F: FnMut(winit::Event)
    {
        self.winit_events_loop.borrow_mut().poll_events(|event| {
            self.handle_event(&event);
            callback(event)
        });
    }

    /// Runs forever until `interrupt()` is called. Whenever an event happens, calls the callback.
    #[inline]
    pub fn run_forever<F>(&mut self, mut callback: F)
        where F: FnMut(winit::Event) -> ControlFlow
    {
        self.winit_events_loop.borrow_mut().run_forever(|event| {
            self.handle_event(&event);
            callback(event)
        })
    }
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {

    pub fn new(events_loop: &EventsLoop,
               _win_attribs: &WindowAttributes,
               pf_reqs: &PixelFormatRequirements,
               opengl: &GlAttributes<&Window>,
               _pl_attribs: &PlatformSpecificWindowBuilderAttributes,
               winit_builder: winit::WindowBuilder)
               -> Result<Self, CreationError> {
        if opengl.sharing.is_some() {
            unimplemented!()
        }

        match opengl.robustness {
            Robustness::RobustNoResetNotification |
            Robustness::RobustLoseContextOnReset => {
                return Err(CreationError::RobustnessNotSupported);
            }
            _ => (),
        }

        let transparent = winit_builder.window.transparent;
        let winit_window = winit_builder.build(&*events_loop.winit_events_loop.borrow()).unwrap();
        let window_id = winit_window.id();
        let view = winit_window.get_nsview() as id;
        let context = match Context::new(view, pf_reqs, opengl, transparent) {
            Ok(context) => Arc::new(context),
            Err(e) => {
                return Err(OsError(format!("Couldn't create OpenGL context: {}", e)));
            }
        };
        let weak_context = Arc::downgrade(&context);

        let new_window = |window_contexts| Window {
            context: context,
            winit_window: winit_window,
            contexts: window_contexts,
        };

        // If a `ContextMap` exists, insert the context for this new window and return it.
        if let Some(window_contexts) = events_loop.window_contexts.lock().unwrap().upgrade() {
            window_contexts.map.lock().unwrap().insert(window_id, weak_context);
            return Ok(new_window(window_contexts));
        }

        // If there is not yet a `ContextMap`, this must be the first window so we must create it.
        let mut map = HashMap::new();
        map.insert(window_id, weak_context);
        let window_contexts = Arc::new(ContextMap { map: Mutex::new(map) });
        *events_loop.window_contexts.lock().unwrap() = Arc::downgrade(&window_contexts);
        Ok(new_window(window_contexts))
    }

    pub fn set_title(&self, title: &str) {
        self.winit_window.set_title(title)
    }

    #[inline]
    pub fn as_winit_window(&self) -> &winit::Window {
        &self.winit_window
    }

    #[inline]
    pub fn as_winit_window_mut(&mut self) -> &mut winit::Window {
        &mut self.winit_window
    }

    pub fn show(&self) {
        self.winit_window.show()
    }

    pub fn hide(&self) {
        self.winit_window.hide()
    }

    pub fn get_position(&self) -> Option<(i32, i32)> {
        self.winit_window.get_position()
    }

    pub fn set_position(&self, x: i32, y: i32) {
        self.winit_window.set_position(x, y)
    }

    pub fn get_inner_size(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size()
    }

    pub fn get_inner_size_points(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size()
    }

    pub fn get_inner_size_pixels(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size().map(|(x, y)| {
            let hidpi = self.hidpi_factor();
            ((x as f32 * hidpi) as u32, (y as f32 * hidpi) as u32)
        })
    }

    pub fn get_outer_size(&self) -> Option<(u32, u32)> {
        self.winit_window.get_outer_size()
    }

    pub fn set_inner_size(&self, x: u32, y: u32) {
        self.winit_window.set_inner_size(x, y)
    }

    #[allow(deprecated)]
    pub unsafe fn platform_display(&self) -> *mut libc::c_void {
        self.winit_window.platform_display()
    }

    #[allow(deprecated)]
    pub unsafe fn platform_window(&self) -> *mut libc::c_void {
        self.winit_window.platform_window()
    }

    pub fn set_cursor(&self, cursor: winit::MouseCursor) {
        self.winit_window.set_cursor(cursor);
    }

    pub fn hidpi_factor(&self) -> f32 {
        self.winit_window.hidpi_factor()
    }

    pub fn set_cursor_position(&self, x: i32, y: i32) -> Result<(), ()> {
        self.winit_window.set_cursor_position(x, y)
    }

    pub fn set_cursor_state(&self, state: winit::CursorState) -> Result<(), String> {
        self.winit_window.set_cursor_state(state)
    }

    pub fn id(&self) -> winit::WindowId {
        self.winit_window.id()
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.contexts.map.lock().unwrap().remove(&self.id());
    }
}

impl Context {
    fn new(view: id,
           pf_reqs: &PixelFormatRequirements,
           opengl: &GlAttributes<&Window>,
           transparent: bool) -> Result<Self, CreationError>
    {
        let attributes = try!(helpers::build_nsattributes(pf_reqs, opengl));
        unsafe {
            let pixelformat = IdRef::new(NSOpenGLPixelFormat::alloc(nil)
                .initWithAttributes_(&attributes));

            if let Some(pixelformat) = pixelformat.non_nil() {

                // TODO: Add context sharing
                let context = IdRef::new(NSOpenGLContext::alloc(nil)
                    .initWithFormat_shareContext_(*pixelformat, nil));

                if let Some(cxt) = context.non_nil() {
                    let pf = {
                        let get_attr = |attrib: appkit::NSOpenGLPixelFormatAttribute| -> i32 {
                            let mut value = 0;

                            NSOpenGLPixelFormat::getValues_forAttribute_forVirtualScreen_(
                                *pixelformat,
                                &mut value,
                                attrib,
                                NSOpenGLContext::currentVirtualScreen(*cxt));

                            value
                        };

                        PixelFormat {
                            hardware_accelerated: get_attr(appkit::NSOpenGLPFAAccelerated) != 0,
                            color_bits: (get_attr(appkit::NSOpenGLPFAColorSize) - get_attr(appkit::NSOpenGLPFAAlphaSize)) as u8,
                            alpha_bits: get_attr(appkit::NSOpenGLPFAAlphaSize) as u8,
                            depth_bits: get_attr(appkit::NSOpenGLPFADepthSize) as u8,
                            stencil_bits: get_attr(appkit::NSOpenGLPFAStencilSize) as u8,
                            stereoscopy: get_attr(appkit::NSOpenGLPFAStereo) != 0,
                            double_buffer: get_attr(appkit::NSOpenGLPFADoubleBuffer) != 0,
                            multisampling: if get_attr(appkit::NSOpenGLPFAMultisample) > 0 {
                                Some(get_attr(appkit::NSOpenGLPFASamples) as u16)
                            } else {
                                None
                            },
                            srgb: true,
                        }
                    };

                    cxt.setView_(view);
                    let value = if opengl.vsync { 1 } else { 0 };
                    cxt.setValues_forParameter_(&value, appkit::NSOpenGLContextParameter::NSOpenGLCPSwapInterval);

                    if transparent {
                        let mut opacity = 0;
                        CGLSetParameter(cxt.CGLContextObj() as *mut _, kCGLCPSurfaceOpacity, &mut opacity);
                    }

                    CGLEnable(cxt.CGLContextObj() as *mut _, kCGLCECrashOnRemovedFunctions);

                    Ok(Context { gl: cxt, pixel_format: pf })
                } else {
                    Err(CreationError::NotSupported)
                }
            } else {
                Err(CreationError::NoAvailablePixelFormat)
            }
        }
    }
}


impl GlContext for Window {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        let _: () = msg_send![*self.context.gl, update];
        self.context.gl.makeCurrentContext();
        Ok(())
    }

    #[inline]
    fn is_current(&self) -> bool {
        unsafe {
            let current = NSOpenGLContext::currentContext(nil);
            if current != nil {
                let is_equal: BOOL = msg_send![current, isEqual:*self.context.gl];
                is_equal != NO
            } else {
                false
            }
        }
    }

    fn get_proc_address(&self, addr: &str) -> *const () {
        let symbol_name: CFString = FromStr::from_str(addr).unwrap();
        let framework_name: CFString = FromStr::from_str("com.apple.opengl").unwrap();
        let framework =
            unsafe { CFBundleGetBundleWithIdentifier(framework_name.as_concrete_TypeRef()) };
        let symbol = unsafe {
            CFBundleGetFunctionPointerForName(framework, symbol_name.as_concrete_TypeRef())
        };
        symbol as *const _
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        unsafe {
            let pool = NSAutoreleasePool::new(nil);
            self.context.gl.flushBuffer();
            let _: () = msg_send![pool, release];
        }
        Ok(())
    }

    #[inline]
    fn get_api(&self) -> ::Api {
        ::Api::OpenGl
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        self.context.pixel_format.clone()
    }
}

struct IdRef(id);

impl IdRef {
    fn new(i: id) -> IdRef {
        IdRef(i)
    }

    #[allow(dead_code)]
    fn retain(i: id) -> IdRef {
        if i != nil {
            let _: id = unsafe { msg_send![i, retain] };
        }
        IdRef(i)
    }

    fn non_nil(self) -> Option<IdRef> {
        if self.0 == nil { None } else { Some(self) }
    }
}

impl Drop for IdRef {
    fn drop(&mut self) {
        if self.0 != nil {
            let _: () = unsafe { msg_send![self.0, release] };
        }
    }
}

impl Deref for IdRef {
    type Target = id;
    fn deref<'a>(&'a self) -> &'a id {
        &self.0
    }
}

impl Clone for IdRef {
    fn clone(&self) -> IdRef {
        if self.0 != nil {
            let _: id = unsafe { msg_send![self.0, retain] };
        }
        IdRef(self.0)
    }
}
