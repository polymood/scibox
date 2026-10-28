//! The shared logic of scibox. The TUI and the GUI use this crate. It does no drawing.

pub mod calc;
#[rustfmt::skip]
pub mod formulas;
pub mod plot;
pub mod search;
pub mod state;
pub mod units;
