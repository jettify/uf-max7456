use embedded_hal_mock::eh1::spi::Mock as SpiMock;
use embedded_hal_mock::eh1::spi::Transaction as SpiTransaction;
use uf_max7456::Max7456Blocking;

fn main() {
    let expectations: &[SpiTransaction<u8>] = &[];
    let spi = SpiMock::new(expectations);

    let driver = Max7456Blocking::new(spi);
    let mut spi = driver.release();
    spi.done();
}
