pub use glutin::*;

#[macro_use]
extern crate log;

pub mod platform;

mod split_context;
mod surface;
mod unified_context;

pub use crate::split_context::*;
pub use crate::surface::*;
pub use crate::unified_context::*;

pub type LighterContextBuilder<'a, IC, PBT, WST, ST> =
    glutin::ContextBuilderWrapper<&'a SplitContext<IC, PBT, WST, ST>>;
