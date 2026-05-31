use embedded_hal::spi::Operation as BlockingOperation;
use embedded_hal::spi::SpiDevice as BlockingSpiDevice;
use embedded_hal_async::spi::Operation as AsyncOperation;
use embedded_hal_async::spi::SpiDevice as AsyncSpiDevice;

use crate::registers;

pub struct Max7456Async<SPI> {
    pub(crate) bus: SPI,
}

impl<SPI> Max7456Async<SPI> {
    pub fn new(bus: SPI) -> Self {
        Self { bus }
    }

    pub fn bus(&mut self) -> &mut SPI {
        &mut self.bus
    }

    pub fn release(self) -> SPI {
        self.bus
    }
}

pub struct Max7456Blocking<SPI> {
    pub(crate) bus: SPI,
}

impl<SPI> Max7456Blocking<SPI> {
    pub fn new(bus: SPI) -> Self {
        Self { bus }
    }

    pub fn bus(&mut self) -> &mut SPI {
        &mut self.bus
    }

    pub fn release(self) -> SPI {
        self.bus
    }
}

impl<SPI> Max7456Async<SPI>
where
    SPI: AsyncSpiDevice,
{
    pub async fn write_register(&mut self, register: u8, value: u8) -> Result<(), SPI::Error> {
        let tx = [register, value];
        let mut ops = [AsyncOperation::Write(&tx)];
        self.bus.transaction(&mut ops).await
    }

    pub async fn read_register(&mut self, register: u8) -> Result<u8, SPI::Error> {
        let mut value = [0_u8];
        let address = [register | registers::READ];
        let mut ops = [
            AsyncOperation::Write(&address),
            AsyncOperation::Read(&mut value),
        ];
        self.bus.transaction(&mut ops).await?;
        Ok(value[0])
    }
}

impl<SPI> Max7456Blocking<SPI>
where
    SPI: BlockingSpiDevice,
{
    pub fn write_register(&mut self, register: u8, value: u8) -> Result<(), SPI::Error> {
        let tx = [register, value];
        let mut ops = [BlockingOperation::Write(&tx)];
        self.bus.transaction(&mut ops)
    }

    pub fn read_register(&mut self, register: u8) -> Result<u8, SPI::Error> {
        let mut value = [0_u8];
        let address = [register | registers::READ];
        let mut ops = [
            BlockingOperation::Write(&address),
            BlockingOperation::Read(&mut value),
        ];
        self.bus.transaction(&mut ops)?;
        Ok(value[0])
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use embedded_hal_mock::eh1::spi::Mock as SpiMock;
    use embedded_hal_mock::eh1::spi::Transaction as SpiTransaction;

    #[test]
    fn test_basic() {
        let expectations: &[SpiTransaction<u8>] = &[];
        let spi = SpiMock::new(expectations);

        let driver = Max7456Blocking::new(spi);
        let mut spi = driver.release();
        spi.done();
    }
}
