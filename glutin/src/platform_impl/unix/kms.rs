#![cfg(feature = "kms")]

use std::{num::NonZeroU32, os::unix::prelude::FromRawFd};

use drm::control::{atomic::AtomicModeReq, property, AtomicCommitFlags, Device, ResourceHandle};
use gbm::{AsRaw, BufferObjectFlags};
use parking_lot::Mutex;
use winit::{
    event_loop::EventLoopWindowTarget,
    platform::unix::{Card, EventLoopWindowTargetExtUnix},
    window::{Window, WindowBuilder},
};

use crate::{
    api::egl::{Egl, NativeDisplay, EGL},
    ContextError, CreationError, GlAttributes, PixelFormat, PixelFormatRequirements, Rect,
};
use glutin_egl_sys as ffi;

use crate::api::egl::Context as EglContext;
use crate::api::egl::SurfaceType as EglSurfaceType;

macro_rules! pf_to_fmt {
    ($pf:expr) => {
        match ($pf.color_bits, $pf.alpha_bits) {
            (Some(24), Some(0) | None) => gbm::Format::Rgb888,
            (Some(16), Some(0) | None) => gbm::Format::Rgb565,
            (Some(8), Some(0) | None) => gbm::Format::Rgb332,
            (Some(15), Some(1)) => gbm::Format::Xrgb1555,
            (Some(30), Some(2)) => gbm::Format::Xrgb2101010,
            (Some(24), Some(8)) => gbm::Format::Xrgb8888,
            (Some(12), Some(4)) => gbm::Format::Xrgb4444,
            _ => gbm::Format::Xrgb8888,
        }
    };
}

#[derive(Debug)]
pub struct CtxLock {
    surface: Option<gbm::Surface<()>>,
    previous_bo: Option<gbm::BufferObject<()>>,
    previous_fb: Option<drm::control::framebuffer::Handle>,
    device: gbm::Device<crate::platform::unix::Card>,
    kms_fence: ffi::egl::types::EGLSyncKHR,
    gpu_fence: ffi::egl::types::EGLSyncKHR,
    kms_in_fence_fd: i32,
    kms_out_fence_fd: i32,
}

unsafe impl Send for CtxLock {}
unsafe impl Sync for CtxLock {}

#[derive(Debug)]
pub struct Context {
    display: EglContext,
    ctx_lock: parking_lot::Mutex<CtxLock>,
    fb_id_property: property::Handle,
    out_fence_ptr_prop: property::Handle,
    in_fence_fd_prop: property::Handle,
    depth: u32,
    bpp: u32,
    plane: drm::control::plane::Handle,
    crtc: drm::control::crtc::Info,
}

impl std::ops::Deref for Context {
    type Target = EglContext;

    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

fn find_prop_id<T: ResourceHandle>(
    card: &Card,
    handle: T,
    name: &'static str,
) -> Option<property::Handle> {
    let props = card.get_properties(handle).expect("Could not get props of connector");
    let (ids, _vals) = props.as_props_and_values();
    ids.iter()
        .find(|&id| {
            let info = card.get_property(*id).unwrap();
            info.name().to_str().map(|x| x == name).unwrap_or(false)
        })
        .cloned()
}

impl Context {
    #[inline]
    pub fn new_headless<T>(
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        _size: Option<winit::dpi::PhysicalSize<u32>>,
    ) -> Result<Self, CreationError> {
        let mut gl_attr = gl_attr.clone().map_sharing(|c| &**c);
        gl_attr.vsync = true;
        let drm_ptr = el
            .drm_device()
            .ok_or(CreationError::NotSupported("GBM is not initialized".into()))?
            .clone();
        let display_ptr =
            gbm::Device::new(drm_ptr).map_err(|e| CreationError::OsError(e.to_string()))?;
        let native_display =
            NativeDisplay::Gbm(Some(display_ptr.as_raw() as ffi::EGLNativeDisplayType));
        let context = EglContext::new(
            pf_reqs,
            &gl_attr,
            native_display,
            EglSurfaceType::Surfaceless,
            |c, _| Ok(c[0]),
        )
        .and_then(|p| p.finish_surfaceless())?;
        let plane =
            el.drm_plane().ok_or(CreationError::NotSupported("GBM is not initialized".into()))?;
        let crtc = el.drm_crtc().ok_or(CreationError::OsError("No crtc found".to_string()))?;
        let context = Context {
            display: context,
            fb_id_property: find_prop_id(&display_ptr, plane, "FB_ID")
                .ok_or(CreationError::NotSupported("Could not get FB_ID".into()))?,
            out_fence_ptr_prop: find_prop_id(&display_ptr, crtc.handle(), "OUT_FENCE_PTR")
                .ok_or(CreationError::NotSupported("Could not get FB_ID".into()))?,
            in_fence_fd_prop: find_prop_id(&display_ptr, plane, "IN_FENCE_FD")
                .ok_or(CreationError::NotSupported("Could not get FB_ID".into()))?,
            ctx_lock: Mutex::new(CtxLock {
                surface: None,
                previous_fb: None,
                previous_bo: None,
                device: display_ptr,
                kms_fence: std::ptr::null(),
                gpu_fence: std::ptr::null(),
                kms_in_fence_fd: -1,
                kms_out_fence_fd: -1,
            }),
            plane,
            crtc: crtc.clone(),
            depth: pf_reqs.depth_bits.unwrap_or(0) as u32,
            bpp: pf_reqs.alpha_bits.unwrap_or(0) as u32 + pf_reqs.color_bits.unwrap_or(0) as u32,
        };
        Ok(context)
    }

    #[inline]
    pub fn new<T>(
        wb: WindowBuilder,
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(Window, Self), CreationError> {
        let window = wb.build(&el)?;
        let size = window.inner_size();
        let (width, height): (u32, u32) = size.into();
        let ctx = Self::new_raw_context(
            el.drm_device()
                .as_ref()
                .ok_or(CreationError::NotSupported("GBM is not initialized".into()))?,
            width,
            height,
            el.drm_plane().ok_or(CreationError::OsError("No plane found".to_string()))?,
            el.drm_crtc().ok_or(CreationError::OsError("No crtc found".to_string()))?.clone(),
            pf_reqs,
            gl_attr,
        )?;
        Ok((window, ctx))
    }

    #[inline]
    pub fn new_raw_context(
        display_ptr: &crate::platform::unix::Card,
        width: u32,
        height: u32,
        plane: drm::control::plane::Handle,
        crtc: drm::control::crtc::Info,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<Self, CreationError> {
        let mut gl_attr = gl_attr.clone().map_sharing(|c| &**c);
        gl_attr.vsync = true;
        let drm_ptr = display_ptr.clone();
        let display_ptr =
            gbm::Device::new(drm_ptr).map_err(|e| CreationError::OsError(e.to_string()))?;
        let format = pf_to_fmt!(pf_reqs);

        let context = EglContext::new(
            pf_reqs,
            &gl_attr,
            NativeDisplay::Gbm(Some(display_ptr.as_raw() as ffi::EGLNativeDisplayType)),
            EglSurfaceType::Window,
            |c, _| Ok(c[0]),
        )?;

        let surface: gbm::Surface<()> = display_ptr
            .create_surface(
                width,
                height,
                format,
                BufferObjectFlags::SCANOUT | BufferObjectFlags::RENDERING,
            )
            .map_err(|e| CreationError::OsError(e.to_string()))?;

        let display = context.finish(surface.as_raw() as ffi::EGLNativeWindowType)?;

        let ctx = Context {
            display,
            fb_id_property: find_prop_id(&display_ptr, plane, "FB_ID")
                .ok_or(CreationError::NotSupported("Could not get FB_ID".into()))?,
            out_fence_ptr_prop: find_prop_id(&display_ptr, crtc.handle(), "OUT_FENCE_PTR")
                .ok_or(CreationError::NotSupported("Could not get FB_ID".into()))?,
            in_fence_fd_prop: find_prop_id(&display_ptr, plane, "IN_FENCE_FD")
                .ok_or(CreationError::NotSupported("Could not get FB_ID".into()))?,
            ctx_lock: Mutex::new(CtxLock {
                surface: Some(surface),
                previous_fb: None,
                previous_bo: None,
                device: display_ptr,
                kms_fence: std::ptr::null(),
                gpu_fence: std::ptr::null(),
                kms_in_fence_fd: -1,
                kms_out_fence_fd: -1,
            }),
            plane,
            crtc: crtc.clone(),
            depth: pf_reqs.depth_bits.unwrap_or(0) as u32,
            bpp: pf_reqs.alpha_bits.unwrap_or(0) as u32 + pf_reqs.color_bits.unwrap_or(0) as u32,
        };
        Ok(ctx)
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        (**self).make_not_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        (**self).is_current()
    }

    #[inline]
    pub fn get_api(&self) -> crate::Api {
        (**self).get_api()
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::EGLContext {
        (**self).raw_handle()
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const std::os::raw::c_void> {
        Some((**self).get_egl_display())
    }

    #[inline]
    pub fn resize(&self, _width: u32, _height: u32) {}

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const core::ffi::c_void {
        (**self).get_proc_address(addr)
    }

    #[inline]
    fn finish_swap_buffers(
        &self,
        egl: &Egl,
        gpu_fence: *const std::os::raw::c_void,
    ) -> Result<(), ContextError> {
        let mut lock = self.ctx_lock.lock();
        lock.gpu_fence = gpu_fence;

        lock.kms_in_fence_fd =
            unsafe { egl.DupNativeFenceFDANDROID(self.display.get_egl_display(), lock.gpu_fence) };

        unsafe {
            egl.DestroySyncKHR(self.display.get_egl_display(), gpu_fence);
        }
        assert!(lock.kms_in_fence_fd != -1);

        let front_buffer = unsafe {
            lock.surface
                .as_ref()
                .ok_or(ContextError::OsError("This context is surfaceless".to_string()))?
                .lock_front_buffer()
                .or_else(|e| {
                    Err(ContextError::OsError(format!("Error locking front buffer: {}", e)))
                })?
        };
        let fb = lock
            .device
            .add_framebuffer(&front_buffer, self.depth, self.bpp)
            .or_else(|e| Err(ContextError::OsError(format!("Error adding framebuffer: {}", e))))?;

        if !lock.kms_fence.is_null() {
            unsafe {
                let mut status: i32 = 0;
                while status != ffi::egl::CONDITION_SATISFIED as i32 {
                    status = egl.ClientWaitSyncKHR(
                        self.display.get_egl_display(),
                        lock.kms_fence,
                        0,
                        ffi::egl::FOREVER,
                    );
                }
                egl.DestroySyncKHR(self.display.get_egl_display(), lock.kms_fence);
            }
        }
        let mut atomic_req = AtomicModeReq::new();
        atomic_req.add_property(
            self.plane,
            self.fb_id_property,
            property::Value::Framebuffer(Some(fb)),
        );
        if lock.kms_in_fence_fd != -1 {
            let fence_ptr: *mut i32 = &mut lock.kms_out_fence_fd;
            atomic_req.add_property(
                self.crtc.handle(),
                self.out_fence_ptr_prop,
                property::Value::Unknown(fence_ptr as u64),
            );
            atomic_req.add_property(
                self.plane,
                self.in_fence_fd_prop,
                property::Value::Object(Some(
                    NonZeroU32::new(lock.kms_in_fence_fd as u32).unwrap(),
                )),
            );
        }
        lock.device
            .atomic_commit(AtomicCommitFlags::NONBLOCK, atomic_req)
            .or_else(|e| Err(ContextError::OsError(format!("Error setting crtc: {}", e))))?;
        if let Some(prev_fb) = lock.previous_fb {
            lock.device.destroy_framebuffer(prev_fb).or_else(|e| {
                Err(ContextError::OsError(format!("Error destroying framebuffer: {}", e)))
            })?
        }
        if lock.kms_in_fence_fd != -1 {
            unsafe {
                drop(std::fs::File::from_raw_fd(lock.kms_in_fence_fd));
            }
            lock.kms_in_fence_fd = -1;
        }
        lock.previous_fb = Some(fb);
        lock.previous_bo = Some(front_buffer);
        lock.gpu_fence = std::ptr::null();
        lock.kms_fence = std::ptr::null();
        if lock.kms_out_fence_fd != -1 {
            let attrib_list = [
                ffi::egl::SYNC_NATIVE_FENCE_FD_ANDROID as i32,
                lock.kms_out_fence_fd,
                ffi::egl::NONE as i32,
            ];

            lock.kms_fence = unsafe {
                egl.CreateSyncKHR(
                    self.display.get_egl_display(),
                    ffi::egl::SYNC_NATIVE_FENCE_ANDROID,
                    attrib_list.as_ptr(),
                )
            };

            assert!(!lock.kms_fence.is_null());
            lock.kms_out_fence_fd = -1;
            unsafe { egl.WaitSyncKHR(self.display.get_egl_display(), lock.kms_fence, 0) };
        }
        Ok(())
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        let egl = EGL.as_ref().unwrap();
        let attrib_list = [
            ffi::egl::SYNC_NATIVE_FENCE_FD_ANDROID as i32,
            ffi::egl::NO_NATIVE_FENCE_FD_ANDROID,
            ffi::egl::NONE as i32,
        ];

        let gpu_fence = unsafe {
            egl.CreateSyncKHR(
                self.display.get_egl_display(),
                ffi::egl::SYNC_NATIVE_FENCE_ANDROID,
                attrib_list.as_ptr(),
            )
        };

        assert!(!gpu_fence.is_null());

        (**self).swap_buffers()?;
        self.finish_swap_buffers(egl, gpu_fence)
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[Rect]) -> Result<(), ContextError> {
        let egl = EGL.as_ref().unwrap();
        let attrib_list = [
            ffi::egl::SYNC_NATIVE_FENCE_FD_ANDROID as i32,
            ffi::egl::NO_NATIVE_FENCE_FD_ANDROID,
            ffi::egl::NONE as i32,
        ];

        let gpu_fence = unsafe {
            egl.CreateSyncKHR(
                self.display.get_egl_display(),
                ffi::egl::SYNC_NATIVE_FENCE_ANDROID,
                attrib_list.as_ptr(),
            )
        };

        assert!(!gpu_fence.is_null());

        (**self).swap_buffers_with_damage(rects)?;
        self.finish_swap_buffers(egl, gpu_fence)
    }

    #[inline]
    pub fn swap_buffers_with_damage_supported(&self) -> bool {
        (**self).swap_buffers_with_damage_supported()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        (**self).get_pixel_format().clone()
    }
}
