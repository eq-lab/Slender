#![deny(warnings)]
#![no_std]

mod fixedi128;
#[cfg(test)]
mod test;

pub use fixedi128::*;

/// Denominator for alpha, used in interest rate calculation
pub const ALPHA_DENOMINATOR: u32 = 100;

/// Percent representation
pub const PERCENTAGE_FACTOR: u32 = 10000;
