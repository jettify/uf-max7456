use embedded_hal::delay::DelayNs as BlockingDelayNs;
use embedded_hal::spi::Operation as BlockingOperation;
use embedded_hal::spi::SpiDevice as BlockingSpiDevice;
use embedded_hal_async::delay::DelayNs as AsyncDelayNs;
use embedded_hal_async::spi::Operation as AsyncOperation;
use embedded_hal_async::spi::SpiDevice as AsyncSpiDevice;

use crate::config::Config;
use crate::error::Error;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DeviceType {
    Max7456,
    At7456e,
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
    pub async fn init<DelayNs>(
        &mut self,
        delay: &mut DelayNs,
        config: Config,
    ) -> Result<DeviceType, Error<SPI::Error>>
    where
        DelayNs: AsyncDelayNs,
    {
        self.write_end_string().await?;

        let osdm = self.read_register(registers::OSDM).await?;
        if osdm != registers::OSDM_DEFAULT {
            return Err(Error::NotFound { osdm });
        }

        self.write_register(registers::CMAL, registers::CMAL_CA8_BIT)
            .await?;
        let cmal = self.read_register(registers::CMAL).await?;
        let device_type = if (cmal & registers::CMAL_CA8_BIT) != 0 {
            DeviceType::At7456e
        } else {
            DeviceType::Max7456
        };

        self.write_register(registers::VM0, registers::VM0_RESET)
            .await?;
        delay.delay_us(config.reset_settle_delay_us).await;
        for _ in 0..config.reset_poll_max_tries {
            let vm0 = self.read_register(registers::VM0).await?;
            if (vm0 & registers::VM0_RESET) == 0 {
                return Ok(device_type);
            }
        }
        Err(Error::ResetTimeout)
    }

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

    async fn write_end_string(&mut self) -> Result<(), SPI::Error> {
        let tx = [registers::END_STRING];
        let mut ops = [AsyncOperation::Write(&tx)];
        self.bus.transaction(&mut ops).await
    }
}

impl<SPI> Max7456Blocking<SPI>
where
    SPI: BlockingSpiDevice,
{
    pub fn init<DelayNs>(
        &mut self,
        delay: &mut DelayNs,
        config: Config,
    ) -> Result<DeviceType, Error<SPI::Error>>
    where
        DelayNs: BlockingDelayNs,
    {
        self.write_end_string()?;

        let osdm = self.read_register(registers::OSDM)?;
        if osdm != registers::OSDM_DEFAULT {
            return Err(Error::NotFound { osdm });
        }

        self.write_register(registers::CMAL, registers::CMAL_CA8_BIT)?;
        let cmal = self.read_register(registers::CMAL)?;
        let device_type = if (cmal & registers::CMAL_CA8_BIT) != 0 {
            DeviceType::At7456e
        } else {
            DeviceType::Max7456
        };

        self.write_register(registers::VM0, registers::VM0_RESET)?;
        delay.delay_us(config.reset_settle_delay_us);
        for _ in 0..config.reset_poll_max_tries {
            let vm0 = self.read_register(registers::VM0)?;
            if (vm0 & registers::VM0_RESET) == 0 {
                return Ok(device_type);
            }
        }
        Err(Error::ResetTimeout)
    }

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

    fn write_end_string(&mut self) -> Result<(), SPI::Error> {
        let tx = [registers::END_STRING];
        let mut ops = [BlockingOperation::Write(&tx)];
        self.bus.transaction(&mut ops)
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use embedded_hal::delay::DelayNs as BlockingDelayNs;
    use embedded_hal_async::delay::DelayNs as AsyncDelayNs;
    use embedded_hal_mock::eh1::spi::Mock as SpiMock;
    use embedded_hal_mock::eh1::spi::Transaction as SpiTransaction;
    use std::vec;

    #[derive(Default)]
    struct NoopDelay;

    impl BlockingDelayNs for NoopDelay {
        fn delay_ns(&mut self, _ns: u32) {}
    }

    #[derive(Default)]
    struct AsyncNoopDelay;
    impl AsyncDelayNs for AsyncNoopDelay {
        async fn delay_ns(&mut self, _ns: u32) {}
    }

    #[test]
    fn test_basic_blocking() {
        let expectations: &[SpiTransaction<u8>] = &[];
        let spi = SpiMock::new(expectations);

        let driver = Max7456Blocking::new(spi);
        let mut spi = driver.release();
        spi.done();
    }
    #[futures_test::test]
    async fn test_basic_async() {
        let expectations: &[SpiTransaction<u8>] = &[];
        let spi = SpiMock::new(expectations);

        let driver = Max7456Async::new(spi);
        let mut spi = driver.release();
        spi.done();
    }

    #[test]
    fn test_init_at7456e_blocking() {
        let expectations: &[SpiTransaction<u8>] = &[
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::END_STRING]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::OSDM | registers::READ]),
            SpiTransaction::read(0x1b),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::CMAL, registers::CMAL_CA8_BIT]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::CMAL | registers::READ]),
            SpiTransaction::read(registers::CMAL_CA8_BIT),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::VM0, registers::VM0_RESET]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::VM0 | registers::READ]),
            SpiTransaction::read(0x00),
            SpiTransaction::transaction_end(),
        ];
        let spi = SpiMock::new(expectations);
        let mut driver = Max7456Blocking::new(spi);
        let mut delay = NoopDelay;
        let device = driver.init(&mut delay, Config::default()).unwrap();
        assert_eq!(device, DeviceType::At7456e);
        let mut spi = driver.release();
        spi.done();
    }

    #[futures_test::test]
    async fn test_init_at7456e_async() {
        let expectations: &[SpiTransaction<u8>] = &[
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::END_STRING]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::OSDM | registers::READ]),
            SpiTransaction::read(0x1b),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::CMAL, registers::CMAL_CA8_BIT]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::CMAL | registers::READ]),
            SpiTransaction::read(registers::CMAL_CA8_BIT),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::VM0, registers::VM0_RESET]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::VM0 | registers::READ]),
            SpiTransaction::read(0x00),
            SpiTransaction::transaction_end(),
        ];
        let spi = SpiMock::new(expectations);
        let mut driver = Max7456Async::new(spi);
        let mut delay = AsyncNoopDelay;
        let device = driver.init(&mut delay, Config::default()).await.unwrap();
        assert_eq!(device, DeviceType::At7456e);
        let mut spi = driver.release();
        spi.done();
    }

    #[test]
    fn test_init_not_found_blocking() {
        let expectations: &[SpiTransaction<u8>] = &[
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::END_STRING]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::OSDM | registers::READ]),
            SpiTransaction::read(0x00),
            SpiTransaction::transaction_end(),
        ];
        let spi = SpiMock::new(expectations);
        let mut driver = Max7456Blocking::new(spi);
        let mut delay = NoopDelay;
        let err = driver.init(&mut delay, Config::default()).err().unwrap();
        assert!(matches!(err, Error::NotFound { osdm: 0x00 }));
        let mut spi = driver.release();
        spi.done();
    }

    #[futures_test::test]
    async fn test_init_not_found_async() {
        let expectations: &[SpiTransaction<u8>] = &[
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::END_STRING]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::OSDM | registers::READ]),
            SpiTransaction::read(0x00),
            SpiTransaction::transaction_end(),
        ];
        let spi = SpiMock::new(expectations);
        let mut driver = Max7456Async::new(spi);
        let mut delay = AsyncNoopDelay;
        let err = driver
            .init(&mut delay, Config::default())
            .await
            .err()
            .unwrap();
        assert!(matches!(err, Error::NotFound { osdm: 0x00 }));
        let mut spi = driver.release();
        spi.done();
    }
}
