#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConfigField {
    BlackLevel,
    WhiteLevel,
    BackgroundGray,
    HOffset,
    VOffset,
    SyncSource,
    VideoMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConfigError {
    OutOfRange {
        field: ConfigField,
        min: i16,
        max: i16,
        got: i16,
    },
    Unsupported {
        field: ConfigField,
    },
}

impl ConfigError {
    pub const fn out_of_range(field: ConfigField, min: i16, max: i16, got: i16) -> Self {
        Self::OutOfRange {
            field,
            min,
            max,
            got,
        }
    }

    pub const fn unsupported(field: ConfigField) -> Self {
        Self::Unsupported { field }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<SpiError> {
    Spi(SpiError),
    NotFound { osdm: u8 },
    ResetTimeout,
    NvrBusyTimeout,
    Uninitialized,
    OutOfBounds { position: u16, len: u16 },
    InvalidCoordinate { x: u8, y: u8, columns: u8, rows: u8 },
    Config(ConfigError),
}

impl<SpiError> From<SpiError> for Error<SpiError> {
    fn from(value: SpiError) -> Self {
        Self::Spi(value)
    }
}
