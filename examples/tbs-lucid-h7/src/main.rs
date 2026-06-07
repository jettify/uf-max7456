#![no_std]
#![no_main]

#[macro_use]
mod fmt;

use embassy_executor::Spawner;
use embassy_time::Delay;
use embassy_time::Timer;
use embedded_hal_bus::spi::ExclusiveDevice;
use tbs_lucid_h7_bsp as bsp;
use uf_max7456::Config;
use uf_max7456::DeviceType;
use uf_max7456::Error;
use uf_max7456::Max7456Async;
use {defmt_rtt as _, panic_probe as _};

const RETRY_MS: u64 = 1_000;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = bsp::hal::init(bsp::config());
    let board = bsp::Board::new(p);

    let mut spi_config = bsp::hal::spi::Config::default();
    spi_config.frequency = bsp::hal::time::mhz(8);

    let osd = board.osd.new_spi(spi_config);
    let cs = bsp::hal::gpio::Output::new(
        osd.cs,
        bsp::hal::gpio::Level::High,
        bsp::hal::gpio::Speed::Low,
    );
    let spi = match ExclusiveDevice::new_no_delay(osd.spi, cs) {
        Ok(spi) => spi,
        Err(_) => loop {
            Timer::after_millis(250).await;
        },
    };
    let max7456 = Max7456Async::new(spi);
    let mut delay = Delay;
    let config = Config::auto();

    match max7456.init(&mut delay, config).await {
        Ok(mut max7456) => {
            let state = max7456.state();
            let name = match state.device_type {
                DeviceType::At7456e => "At7456e",
                DeviceType::Max7456 => "Max7456",
            };
            info!(
                "Detected OSD chip: {}, mode={:?}, rows={}, cols={}",
                name, state.video_mode, state.geometry.rows, state.geometry.columns
            );
            match max7456.write_text(0, 0, b"uf-max7456").await {
                Ok(()) => info!("OSD write ok"),
                Err(_) => info!("OSD write failed"),
            }
        }
        Err(init_err) => {
            let (_max7456, error) = init_err.into_parts();
            match error {
                Error::NotFound { osdm } => info!("OSD chip not detected, OSDM=0x{:02x}", osdm),
                Error::ResetTimeout => info!("OSD reset timeout"),
                Error::NvrBusyTimeout => info!("OSD NVR busy timeout"),
                Error::Spi(_) => info!("OSD SPI transaction failed"),
                Error::Config(_) => info!("InvalidConfig"),
                Error::OutOfBounds { .. } | Error::InvalidCoordinate { .. } => {
                    info!("OSD internal state error")
                }
            }
        }
    }

    loop {
        Timer::after_millis(RETRY_MS).await;
    }
}
