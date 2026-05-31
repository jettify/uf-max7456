#![no_std]
#![allow(
    clippy::needless_doctest_main,
    reason = "This is readme example, not doctest"
)]
#![doc = include_str!("../README.md")]

pub mod bus;
pub mod config;
pub mod error;
pub mod registers;

pub use bus::DeviceType;
pub use bus::Max7456Async;
pub use bus::Max7456Blocking;
pub use config::Config;
pub use error::Error;
