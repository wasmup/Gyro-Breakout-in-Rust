#![no_std]

pub mod fractal;
pub mod game;
pub mod input;
pub mod render;
pub mod scores;

#[cfg(test)]
extern crate self as gyro_breakout;
#[cfg(test)]
extern crate std;
