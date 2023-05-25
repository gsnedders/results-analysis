#![warn(clippy::enum_glob_use)]
#![warn(clippy::float_arithmetic)]
#![warn(clippy::from_iter_instead_of_collect)]
#![warn(clippy::unnested_or_patterns)]

extern crate approx;
extern crate serde_derive;

pub mod bsf;
pub mod utils;
pub mod wptreport;

pub use bsf::score_test;
