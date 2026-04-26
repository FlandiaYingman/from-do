#![doc = include_str!("recur/Grammar.md")]

mod next;
mod normalize;
pub mod pattern;
pub mod strfrecur;
pub mod strprecur;

pub use pattern::*;
pub use strfrecur::*;
pub use strprecur::*;

#[cfg(test)]
mod test_roundtrip;
