#![allow(clippy::op_ref)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod action;
pub mod actions;
pub mod collection;
pub mod deadlock;
pub mod direction;
pub mod error;
pub mod level;
pub mod map;
pub mod math;
pub mod path_finding;
pub mod run_length;
pub mod solver;
pub mod tiles;

mod node;
mod state;

pub use action::*;
pub use actions::*;
pub use collection::*;
pub use direction::*;
pub use error::*;
pub use level::*;
pub use map::*;
pub use math::*;
pub use tiles::*;
