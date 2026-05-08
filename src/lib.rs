#![allow(clippy::op_ref)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod action;
pub mod actions;
pub mod box_set;
pub mod collection;
pub mod deadlock;
pub mod direction;
pub mod error;
pub mod level;
pub mod map;
pub mod matching;
pub mod math;
pub mod path_finding;
pub mod run_length;
pub mod solver;
pub mod tiles;

pub use action::*;
pub use actions::*;
pub use box_set::*;
pub use collection::*;
pub use direction::*;
pub use error::*;
pub use level::*;
pub use map::*;
pub use math::*;
pub use tiles::*;

/// Convenience re-exports for `use sokoban_core::prelude::*;`.
///
/// Brings the types you almost always need — `Map`, `Level`, `Action`,
/// `IVector2`, etc. — into scope, plus the [`solver::Solver`] entry point
/// and its associated configuration types.
pub mod prelude {
    pub use crate::{
        action::Action,
        actions::Actions,
        collection::Collection,
        direction::Direction,
        error::{ActionError, ParseActionError, ParseLevelError, ParseMapError, SearchError},
        level::Level,
        map::Map,
        math::IVector2,
        solver::{Solver, Strategy, Terminator},
        tiles::Tiles,
    };
}
