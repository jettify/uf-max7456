#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<SpiError> {
    Spi(SpiError),
    NotFound { osdm: u8 },
    ResetTimeout,
}

impl<SpiError> From<SpiError> for Error<SpiError> {
    fn from(value: SpiError) -> Self {
        Self::Spi(value)
    }
}
