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
pub use bus::DisplayPosition;
pub use bus::Max7456Async;
pub use bus::Max7456Blocking;
pub use bus::RuntimeState;
pub use bus::Status;
pub use config::BackgroundMode;
pub use config::Config;
pub use config::ScreenGeometry;
pub use config::SyncSource;
pub use config::VideoMode;
pub use error::ConfigError;
pub use error::ConfigField;
pub use error::Error;
