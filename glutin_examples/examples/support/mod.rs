#![allow(dead_code)]

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
use glutin::platform::unix::osmesa::{OsMesaBuffer, OsMesaContext, OsMesaContextBuilder};

use glutin::config::{Api, Config, ConfigsFinder, Version};
use glutin::context::{Context, ContextBuilder};
use glutin::surface::{PBuffer, Surface};
use winit::event_loop::EventLoop;
use winit_types::dpi::PhysicalSize;
use winit_types::error::{Error, ErrorType};

use std::ffi::{c_void, CStr};

pub mod gl {
    pub use self::Gles2 as Gl;
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

pub struct Gl {
    pub gl: gl::Gl,
}

impl Gl {
    pub fn load<F>(loadfn: F) -> Self
    where
        F: FnMut(&'static str) -> *const c_void,
    {
        let gl = gl::Gl::load_with(loadfn);

        let version = unsafe {
            let data = CStr::from_ptr(gl.GetString(gl::VERSION) as *const _)
                .to_bytes()
                .to_vec();
            String::from_utf8(data).unwrap()
        };

        println!("OpenGL version {}", version);

        unsafe {
            let vs = gl.CreateShader(gl::VERTEX_SHADER);
            gl.ShaderSource(
                vs,
                1,
                [VS_SRC.as_ptr() as *const _].as_ptr(),
                std::ptr::null(),
            );
            gl.CompileShader(vs);

            let fs = gl.CreateShader(gl::FRAGMENT_SHADER);
            gl.ShaderSource(
                fs,
                1,
                [FS_SRC.as_ptr() as *const _].as_ptr(),
                std::ptr::null(),
            );
            gl.CompileShader(fs);

            let program = gl.CreateProgram();
            gl.AttachShader(program, vs);
            gl.AttachShader(program, fs);
            gl.LinkProgram(program);
            gl.UseProgram(program);

            let mut vb = 0;
            gl.GenBuffers(1, &mut vb);
            gl.BindBuffer(gl::ARRAY_BUFFER, vb);
            gl.BufferData(
                gl::ARRAY_BUFFER,
                (VERTEX_DATA.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
                VERTEX_DATA.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            if gl.BindVertexArray.is_loaded() {
                let mut vao = 0;
                gl.GenVertexArrays(1, &mut vao);
                gl.BindVertexArray(vao);
            }

            let pos_attrib = gl.GetAttribLocation(program, b"position\0".as_ptr() as *const _);
            let color_attrib = gl.GetAttribLocation(program, b"color\0".as_ptr() as *const _);
            gl.VertexAttribPointer(
                pos_attrib as gl::types::GLuint,
                2,
                gl::FLOAT,
                0,
                5 * std::mem::size_of::<f32>() as gl::types::GLsizei,
                std::ptr::null(),
            );
            gl.VertexAttribPointer(
                color_attrib as gl::types::GLuint,
                3,
                gl::FLOAT,
                0,
                5 * std::mem::size_of::<f32>() as gl::types::GLsizei,
                (2 * std::mem::size_of::<f32>()) as *const () as *const _,
            );
            gl.EnableVertexAttribArray(pos_attrib as gl::types::GLuint);
            gl.EnableVertexAttribArray(color_attrib as gl::types::GLuint);
        }

        Gl { gl }
    }

    pub fn draw_frame(&self, color: [f32; 4]) {
        unsafe {
            self.gl.ClearColor(color[0], color[1], color[2], color[3]);
            self.gl.Clear(gl::COLOR_BUFFER_BIT);
            self.gl.DrawArrays(gl::TRIANGLES, 0, 3);
        }
    }

    pub fn make_renderbuf(&self, size: PhysicalSize<u32>) -> gl::types::GLuint {
        unsafe {
            let mut render_buf = 0;
            self.gl.GenRenderbuffers(1, &mut render_buf);
            self.gl.BindRenderbuffer(gl::RENDERBUFFER, render_buf);
            self.gl.RenderbufferStorage(
                gl::RENDERBUFFER,
                gl::RGB8,
                size.width as _,
                size.height as _,
            );
            render_buf
        }
    }

    pub fn make_framebuffer(&self, render_buf: gl::types::GLuint) -> gl::types::GLuint {
        unsafe {
            let mut fb = 0;
            self.gl.GenFramebuffers(1, &mut fb);
            self.gl.BindFramebuffer(gl::FRAMEBUFFER, fb);
            self.gl.BindRenderbuffer(gl::RENDERBUFFER, render_buf);
            self.gl.FramebufferRenderbuffer(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::RENDERBUFFER,
                render_buf,
            );

            fb
        }
    }
}

#[rustfmt::skip]
static VERTEX_DATA: [f32; 15] = [
    -0.5, -0.5,  1.0,  0.0,  0.0,
     0.0,  0.5,  0.0,  1.0,  0.0,
     0.5, -0.5,  0.0,  0.0,  1.0,
];

const VS_SRC: &'static [u8] = b"
#version 100
precision mediump float;

attribute vec2 position;
attribute vec3 color;

varying vec3 v_color;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_color = color;
}
\0";

const FS_SRC: &'static [u8] = b"
#version 100
precision mediump float;

varying vec3 v_color;

void main() {
    gl_FragColor = vec4(v_color, 1.0);
}
\0";

pub enum HeadlessBackend {
    Surfaceless(Context),
    PBuffer(Context, Surface<PBuffer>),
    OsMesa(OsMesaContext, OsMesaBuffer),
}

impl HeadlessBackend {
    pub fn new<T>(
        el: &EventLoop<T>,
        size: &PhysicalSize<u32>,
        must_support_windows: bool,
    ) -> Result<(Self, Option<Config>), Error> {
        let mut errs = winit_types::make_error!(ErrorType::NotSupported(
            "None of the backends seem to work!".to_string()
        ));

        match ConfigsFinder::new()
            .with_must_support_surfaceless(true)
            .with_must_support_windows(must_support_windows)
            .with_gl((Api::OpenGl, Version(3, 0)))
            .find(&**el)
        {
            Ok(mut confs) => {
                let conf = confs.drain(..1).next().unwrap();
                println!("Surfaceless configeration chosen: {:?}", conf);
                let ctx = ContextBuilder::new().build(&conf).unwrap();
                return Ok((HeadlessBackend::Surfaceless(ctx), Some(conf)));
            }
            Err(err) => errs.append(err),
        }

        match ConfigsFinder::new()
            .with_must_support_pbuffers(true)
            .with_must_support_windows(must_support_windows)
            .with_gl((Api::OpenGl, Version(3, 0)))
            .find(&**el)
        {
            Ok(mut confs) => {
                let conf = confs.drain(..1).next().unwrap();
                println!("PBuffer configeration chosen: {:?}", conf);

                let ctx = ContextBuilder::new().build(&conf).unwrap();
                let surf = unsafe { Surface::new_pbuffer(&conf, size).unwrap() };

                return Ok((HeadlessBackend::PBuffer(ctx, surf), Some(conf)));
            }
            Err(err) => errs.append(err),
        }

        if must_support_windows {
            return Err(errs);
        }

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
        ))]
        match OsMesaContextBuilder::new().build(Version(3, 0)) {
            Ok(ctx) => match OsMesaBuffer::new(size) {
                Ok(buf) => {
                    println!("OsMesa chosen");
                    return Ok((HeadlessBackend::OsMesa(ctx, buf), None));
                }
                Err(err) => errs.append(err),
            },
            Err(err) => errs.append(err),
        }

        return Err(errs);
    }

    pub unsafe fn make_current(&self) -> Result<(), Error> {
        match self {
            HeadlessBackend::Surfaceless(ctx) => ctx.make_current_surfaceless(),
            HeadlessBackend::PBuffer(ctx, surf) => ctx.make_current(surf),
            HeadlessBackend::OsMesa(ctx, buf) => ctx.make_current(buf),
        }
    }

    pub fn load_symbols(&self) -> Result<Gl, Error> {
        unsafe {
            self.make_current()?;
        }
        let gl = match self {
            HeadlessBackend::Surfaceless(ctx) | HeadlessBackend::PBuffer(ctx, _) => {
                Gl::load(|s| ctx.get_proc_address(s).unwrap())
            }
            HeadlessBackend::OsMesa(ctx, _) => Gl::load(|s| ctx.get_proc_address(s).unwrap()),
        };
        Ok(gl)
    }

    pub fn context(&self) -> &Context {
        match self {
            HeadlessBackend::Surfaceless(ctx) | HeadlessBackend::PBuffer(ctx, _) => ctx,
            _ => panic!(),
        }
    }
}
