//! The glutin prelude.
//!
//! The purpose of this module is to bring common imports, given that all
//! graphics api are on the traits for the documetation sharing purposes.
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
