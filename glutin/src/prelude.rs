//! The glutin prelude.
//!
//! The purpose of this module is to make accessing common imports more
//! convenient. The prelude also imports traits shared by the implementations of
//! graphics apis.
//!
//! ```no_run
//! # #![allow(unused_imports)]
//! use glutin::prelude::*;
//! ```

pub use crate::config::GlConfig;
pub use crate::context::{
    NotCurrentGlContext, NotCurrentGlContextSurfaceAccessor,
    PossiblyCurrentContextGlSurfaceAccessor, PossiblyCurrentGlContext,
};
pub use crate::display::GlDisplay;
pub use crate::surface::GlSurface;

// TODO(breaking release) - make pub.
pub(crate) use crate::context::GlContext;
