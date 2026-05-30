# uf-max7456

[![CI](https://github.com/jettify/uf-max7456/actions/workflows/CI.yml/badge.svg)](https://github.com/jettify/uf-max7456/actions/workflows/CI.yml)
[![codecov](https://codecov.io/gh/jettify/uf-max7456/graph/badge.svg?token=XZK0JJQ9QN)](https://codecov.io/gh/jettify/uf-max7456)
[![crates.io](https://img.shields.io/crates/v/uf-max7456)](https://crates.io/crates/uf-max7456)
[![docs.rs](https://img.shields.io/docsrs/uf-max7456)](https://docs.rs/uf-max7456/latest/uf_max7456/)

`uf-max7456` is a platform-agnostic, `no_std` driver for the `MAX7456` and `AT7456E`.

Originally developed for the `uflight` flight-controller project, but useful as a standalone driver.

## Supported Hardware

- MAX7456
- AT7456E

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


## License

This project is licensed under `Apache-2.0`. See `LICENSE` for details.
