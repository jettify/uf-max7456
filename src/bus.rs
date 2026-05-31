pub struct Max7456<BUS> {
    pub(crate) bus: BUS,
}

impl<BUS> Max7456<BUS> {
    pub fn new(bus: BUS) -> Self {
        Self { bus }
    }
    /// Get a reference to the bus
    pub fn bus(&mut self) -> &mut BUS {
        &mut self.bus
    }

    /// Release the bus from the ICM42688 instance
    pub fn release(self) -> BUS {
        self.bus
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use embedded_hal_mock::eh1::spi::{Mock as SpiMock, Transaction as SpiTransaction};

    #[test]
    fn test_basic() {
        let expectations: &[SpiTransaction<u8>] = &[];
        let spi = SpiMock::new(expectations);

        let driver = Max7456::new(spi);
        let mut spi = driver.release();
        spi.done();
    }
}
