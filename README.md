# uf-max7456

[![CI](https://github.com/jettify/uf-max7456/actions/workflows/CI.yml/badge.svg)](https://github.com/jettify/uf-max7456/actions/workflows/CI.yml)
[![codecov](https://codecov.io/gh/jettify/uf-max7456/graph/badge.svg?token=XZK0JJQ9QN)](https://codecov.io/gh/jettify/uf-max7456)
[![crates.io](https://img.shields.io/crates/v/uf-max7456)](https://crates.io/crates/uf-max7456)
[![docs.rs](https://img.shields.io/docsrs/uf-max7456)](https://docs.rs/uf-max7456/latest/uf_max7456/)

`uf-max7456` is a platform-agnostic, `no_std` driver for the `MAX7456` and `AT7456E` single-channel monochrome on-screen display,
often found in betaflight/inav compatible flight controllers.

## Note
Originally developed for the `uflight` flight-controller project, but useful as a standalone driver.
Library is under active development and testing, API might change at any time.

## Supported Hardware

- [`MAX7456`](https://www.analog.com/media/en/technical-documentation/data-sheets/max7456.pdf)
- [`AT7456E`](https://www.mouser.com/pdfdocs/AT7546.pdf)

## Installation

```toml
[dependencies]
uf-max7456 = "0.1"
```

Enable `defmt` formatting support with:

```toml
[dependencies]
uf-max7456 = { version = "0.1", features = ["defmt"] }
```

## Usage

```rust
use uf_max7456::{Config, Configured, Max7456Async};

async fn init_osd<SPI, DELAY>(
    spi: SPI,
    delay: &mut DELAY,
) -> Result<Max7456Async<SPI, Configured>, uf_max7456::Error<SPI::Error>>
where
    SPI: embedded_hal_async::spi::SpiDevice,
    DELAY: embedded_hal_async::delay::DelayNs,
{
    let osd = Max7456Async::new(spi);
    let mut osd = osd.init(delay, Config::auto()).await.map_err(|e| e.error)?;
    let state = osd.state();

    osd.write_text(0, 0, b"uf-max7456").await?;
    osd.write_char(state.geometry.columns - 1, 0, b'!').await?;

    Ok(osd)
}
```


## License

This project is licensed under `Apache-2.0`. See `LICENSE` for details.
