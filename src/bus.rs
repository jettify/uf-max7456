use embedded_hal::delay::DelayNs as BlockingDelayNs;
use embedded_hal::spi::Operation as BlockingOperation;
use embedded_hal::spi::SpiDevice as BlockingSpiDevice;
use embedded_hal_async::delay::DelayNs as AsyncDelayNs;
use embedded_hal_async::spi::Operation as AsyncOperation;
use embedded_hal_async::spi::SpiDevice as AsyncSpiDevice;

use crate::config::BackgroundMode;
use crate::config::Config;
use crate::config::ScreenGeometry;
use crate::config::SyncSource;
use crate::config::VideoMode;
use crate::error::ConfigError;
use crate::error::ConfigField;
use crate::error::Error;
use crate::registers;

const NVR_BUSY_POLL_TRIES: u16 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DeviceType {
    Max7456,
    At7456e,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RuntimeState {
    pub device_type: DeviceType,
    pub video_mode: VideoMode,
    pub geometry: ScreenGeometry,
    pub vm0: u8,
    pub vm1: u8,
    pub dmm: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DisplayPosition {
    index: u16,
}

impl DisplayPosition {
    pub const fn index(self) -> u16 {
        self.index
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Status {
    raw: u8,
}

impl Status {
    pub const fn from_raw(raw: u8) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> u8 {
        self.raw
    }

    pub const fn video_lost(self) -> bool {
        (self.raw & registers::STAT_LOS) != 0
    }

    pub const fn pal(self) -> bool {
        !self.video_lost() && (self.raw & registers::STAT_PAL) != 0
    }

    pub const fn ntsc(self) -> bool {
        !self.video_lost() && (self.raw & registers::STAT_NTSC) != 0
    }

    pub const fn nvr_busy(self) -> bool {
        (self.raw & registers::STAT_NVR_BUSY) != 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Uninitialized;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Configured {
    runtime: RuntimeState,
}

#[derive(Debug)]
pub struct InitError<Driver, SpiError> {
    pub driver: Driver,
    pub error: Error<SpiError>,
}

impl<Driver, SpiError> InitError<Driver, SpiError> {
    pub fn into_parts(self) -> (Driver, Error<SpiError>) {
        (self.driver, self.error)
    }

    pub fn error(&self) -> &Error<SpiError> {
        &self.error
    }

    pub fn driver(&self) -> &Driver {
        &self.driver
    }
}

#[derive(Debug)]
pub struct Max7456Async<SPI, State = Uninitialized> {
    pub(crate) bus: SPI,
    state: State,
}

impl<SPI> Max7456Async<SPI, Uninitialized> {
    pub fn new(bus: SPI) -> Self {
        Self {
            bus,
            state: Uninitialized,
        }
    }
}

impl<SPI, State> Max7456Async<SPI, State> {
    pub fn bus(&mut self) -> &mut SPI {
        &mut self.bus
    }

    pub fn release(self) -> SPI {
        self.bus
    }
}

impl<SPI> Max7456Async<SPI, Configured> {
    pub fn state(&self) -> RuntimeState {
        self.state.runtime
    }

    pub fn geometry(&self) -> ScreenGeometry {
        self.state.runtime.geometry
    }

    pub fn position(
        &self,
        x: u8,
        y: u8,
    ) -> Result<DisplayPosition, Error<core::convert::Infallible>> {
        display_position(self.geometry(), x, y)
    }
}

#[derive(Debug)]
pub struct Max7456Blocking<SPI, State = Uninitialized> {
    pub(crate) bus: SPI,
    state: State,
}

impl<SPI> Max7456Blocking<SPI, Uninitialized> {
    pub fn new(bus: SPI) -> Self {
        Self {
            bus,
            state: Uninitialized,
        }
    }
}

impl<SPI, State> Max7456Blocking<SPI, State> {
    pub fn bus(&mut self) -> &mut SPI {
        &mut self.bus
    }

    pub fn release(self) -> SPI {
        self.bus
    }
}

impl<SPI> Max7456Blocking<SPI, Configured> {
    pub fn state(&self) -> RuntimeState {
        self.state.runtime
    }

    pub fn geometry(&self) -> ScreenGeometry {
        self.state.runtime.geometry
    }

    pub fn position(
        &self,
        x: u8,
        y: u8,
    ) -> Result<DisplayPosition, Error<core::convert::Infallible>> {
        display_position(self.geometry(), x, y)
    }
}

impl<SPI, State> Max7456Async<SPI, State>
where
    SPI: AsyncSpiDevice,
{
    fn init_error(self, error: Error<SPI::Error>) -> InitError<Self, SPI::Error> {
        InitError {
            driver: self,
            error,
        }
    }

    fn init_spi_error(self, error: SPI::Error) -> InitError<Self, SPI::Error> {
        self.init_error(Error::Spi(error))
    }

    pub async fn raw_write_register(&mut self, register: u8, value: u8) -> Result<(), SPI::Error> {
        let tx = [register, value];
        let mut ops = [AsyncOperation::Write(&tx)];
        self.bus.transaction(&mut ops).await
    }

    pub async fn raw_read_register(&mut self, register: u8) -> Result<u8, SPI::Error> {
        let mut value = [0_u8];
        let address = [register | registers::READ];
        let mut ops = [
            AsyncOperation::Write(&address),
            AsyncOperation::Read(&mut value),
        ];
        self.bus.transaction(&mut ops).await?;
        Ok(value[0])
    }

    pub async fn read_status(&mut self) -> Result<Status, SPI::Error> {
        self.raw_read_register(registers::STAT)
            .await
            .map(Status::from_raw)
    }

    async fn write_end_string(&mut self) -> Result<(), SPI::Error> {
        let tx = [registers::END_STRING];
        let mut ops = [AsyncOperation::Write(&tx)];
        self.bus.transaction(&mut ops).await
    }

    async fn detect_device_type(&mut self) -> Result<DeviceType, Error<SPI::Error>> {
        self.raw_write_register(registers::VM0, 0).await?;
        self.wait_nvr_idle(NVR_BUSY_POLL_TRIES).await?;
        for _ in 0..3 {
            self.raw_write_register(registers::CMAL, registers::CMAL_CA8_BIT)
                .await?;
            let cmal = self.raw_read_register(registers::CMAL).await?;
            if (cmal & registers::CMAL_CA8_BIT) != 0 {
                return Ok(DeviceType::At7456e);
            }
        }
        Ok(DeviceType::Max7456)
    }

    async fn wait_nvr_idle(&mut self, max_tries: u16) -> Result<(), Error<SPI::Error>> {
        for _ in 0..max_tries {
            let stat = self.read_status().await?;
            if !stat.nvr_busy() {
                return Ok(());
            }
        }
        Err(Error::NvrBusyTimeout)
    }
}

impl<SPI> Max7456Async<SPI, Uninitialized>
where
    SPI: AsyncSpiDevice,
{
    pub async fn init<DelayNs>(
        mut self,
        delay: &mut DelayNs,
        config: Config,
    ) -> Result<Max7456Async<SPI, Configured>, InitError<Self, SPI::Error>>
    where
        DelayNs: AsyncDelayNs,
    {
        if let Err(e) = self.write_end_string().await {
            return Err(self.init_spi_error(e));
        }

        let osdm = match self.raw_read_register(registers::OSDM).await {
            Ok(v) => v,
            Err(e) => return Err(self.init_spi_error(e)),
        };
        if osdm != registers::OSDM_DEFAULT {
            return Err(self.init_error(Error::NotFound { osdm }));
        }

        let device_type = match self.detect_device_type().await {
            Ok(v) => v,
            Err(e) => return Err(self.init_error(e)),
        };

        if let Err(e) = self
            .raw_write_register(registers::VM0, registers::VM0_RESET)
            .await
        {
            return Err(self.init_spi_error(e));
        }
        delay.delay_us(config.reset_settle_delay_us).await;
        for _ in 0..config.reset_poll_max_tries {
            let vm0 = match self.raw_read_register(registers::VM0).await {
                Ok(v) => v,
                Err(e) => return Err(self.init_spi_error(e)),
            };
            if (vm0 & registers::VM0_RESET) == 0 {
                let state = match self.apply_config(device_type, config).await {
                    Ok(v) => v,
                    Err(e) => return Err(self.init_error(e)),
                };
                return Ok(Max7456Async {
                    bus: self.bus,
                    state: Configured { runtime: state },
                });
            }
        }
        Err(self.init_error(Error::ResetTimeout))
    }

    async fn apply_config(
        &mut self,
        device_type: DeviceType,
        config: Config,
    ) -> Result<RuntimeState, Error<SPI::Error>> {
        validate_config(config)?;

        let video_mode = resolve_video_mode_async(self, config.video_mode).await?;
        let state = runtime_state_from_config(device_type, config, video_mode)?;
        self.raw_write_register(registers::VM0, state.vm0).await?;
        self.raw_write_register(registers::HOS, hos_from_offset(config.h_offset))
            .await?;
        self.raw_write_register(registers::VOS, vos_from_offset(config.v_offset))
            .await?;

        self.raw_write_register(registers::VM1, state.vm1).await?;
        self.raw_write_register(registers::DMM, state.dmm).await?;

        let rb = brightness_register(config.black_level, config.white_level)?;
        for register in registers::RB0..=registers::RB15 {
            self.raw_write_register(register, rb).await?;
        }
        Ok(state)
    }
}

impl<SPI> Max7456Async<SPI, Configured>
where
    SPI: AsyncSpiDevice,
{
    pub async fn set_invert(&mut self, invert: bool) -> Result<(), Error<SPI::Error>> {
        let mut state = self.state.runtime;
        state.dmm = toggled_dmm(state.dmm, invert);
        self.raw_write_register(registers::DMM, state.dmm).await?;
        self.state.runtime = state;
        Ok(())
    }

    pub async fn clear_display(&mut self) -> Result<(), Error<SPI::Error>> {
        let dmm = self.state.runtime.dmm;
        self.raw_write_register(registers::DMM, dmm | registers::DMM_CLEAR_DISPLAY)
            .await?;
        self.restore_dmm().await
    }

    pub async fn write_display_byte(
        &mut self,
        position: DisplayPosition,
        value: u8,
    ) -> Result<(), Error<SPI::Error>> {
        self.set_display_address(position.index()).await?;
        self.raw_write_register(registers::DMDI, sanitize_display_byte(value))
            .await?;
        Ok(())
    }

    pub async fn write_display_run_at(
        &mut self,
        position: DisplayPosition,
        values: &[u8],
    ) -> Result<(), Error<SPI::Error>> {
        let len = display_slice_len(values)?;
        self.check_display_range(position.index(), len)?;
        if values.is_empty() {
            return Ok(());
        }
        if let [value] = values {
            return self.write_display_byte(position, *value).await;
        }

        self.write_dmm_mode(self.state.runtime.dmm | registers::DMM_AUTO_INCREMENT)
            .await?;
        self.set_display_address(position.index()).await?;
        for value in values {
            self.raw_write_register(registers::DMDI, sanitize_display_byte(*value))
                .await?;
        }
        self.write_end_string().await?;
        self.restore_dmm().await
    }

    pub async fn write_char(&mut self, x: u8, y: u8, value: u8) -> Result<(), Error<SPI::Error>> {
        let position = self.display_position(x, y)?;
        self.write_display_byte(position, value).await
    }

    pub async fn write_text(
        &mut self,
        x: u8,
        y: u8,
        values: &[u8],
    ) -> Result<(), Error<SPI::Error>> {
        let position = write_text_position(self.state.runtime.geometry, x, y, values)?;
        self.write_display_run_at(position, values).await
    }

    pub async fn clear_row(&mut self, y: u8) -> Result<(), Error<SPI::Error>> {
        let state = self.state.runtime;
        let position = display_position(state.geometry, 0, y)?;
        let spaces = [b' '; 30];
        self.write_display_run_at(position, &spaces).await
    }

    async fn set_display_address(&mut self, position: u16) -> Result<(), SPI::Error> {
        self.raw_write_register(registers::DMAH, (position >> 8) as u8)
            .await?;
        self.raw_write_register(registers::DMAL, position as u8)
            .await
    }

    async fn write_dmm_mode(&mut self, value: u8) -> Result<(), SPI::Error> {
        self.raw_write_register(registers::DMM, value).await
    }

    async fn restore_dmm(&mut self) -> Result<(), Error<SPI::Error>> {
        let dmm = self.state.runtime.dmm;
        self.write_dmm_mode(dmm).await?;
        Ok(())
    }

    fn display_position(&self, x: u8, y: u8) -> Result<DisplayPosition, Error<SPI::Error>> {
        display_position(self.state.runtime.geometry, x, y)
    }

    fn check_display_range(&self, position: u16, len: u16) -> Result<(), Error<SPI::Error>> {
        check_display_range(self.state.runtime.geometry, position, len)
    }
}

impl<SPI, State> Max7456Blocking<SPI, State>
where
    SPI: BlockingSpiDevice,
{
    fn init_error(self, error: Error<SPI::Error>) -> InitError<Self, SPI::Error> {
        InitError {
            driver: self,
            error,
        }
    }

    fn init_spi_error(self, error: SPI::Error) -> InitError<Self, SPI::Error> {
        self.init_error(Error::Spi(error))
    }

    pub fn raw_write_register(&mut self, register: u8, value: u8) -> Result<(), SPI::Error> {
        let tx = [register, value];
        let mut ops = [BlockingOperation::Write(&tx)];
        self.bus.transaction(&mut ops)
    }

    pub fn raw_read_register(&mut self, register: u8) -> Result<u8, SPI::Error> {
        let mut value = [0_u8];
        let address = [register | registers::READ];
        let mut ops = [
            BlockingOperation::Write(&address),
            BlockingOperation::Read(&mut value),
        ];
        self.bus.transaction(&mut ops)?;
        Ok(value[0])
    }

    pub fn read_status(&mut self) -> Result<Status, SPI::Error> {
        self.raw_read_register(registers::STAT)
            .map(Status::from_raw)
    }

    fn write_end_string(&mut self) -> Result<(), SPI::Error> {
        let tx = [registers::END_STRING];
        let mut ops = [BlockingOperation::Write(&tx)];
        self.bus.transaction(&mut ops)
    }

    fn detect_device_type(&mut self) -> Result<DeviceType, Error<SPI::Error>> {
        self.raw_write_register(registers::VM0, 0)?;
        self.wait_nvr_idle(NVR_BUSY_POLL_TRIES)?;
        for _ in 0..3 {
            self.raw_write_register(registers::CMAL, registers::CMAL_CA8_BIT)?;
            let cmal = self.raw_read_register(registers::CMAL)?;
            if (cmal & registers::CMAL_CA8_BIT) != 0 {
                return Ok(DeviceType::At7456e);
            }
        }
        Ok(DeviceType::Max7456)
    }

    fn wait_nvr_idle(&mut self, max_tries: u16) -> Result<(), Error<SPI::Error>> {
        for _ in 0..max_tries {
            let stat = self.read_status()?;
            if !stat.nvr_busy() {
                return Ok(());
            }
        }
        Err(Error::NvrBusyTimeout)
    }
}

impl<SPI> Max7456Blocking<SPI, Uninitialized>
where
    SPI: BlockingSpiDevice,
{
    pub fn init<DelayNs>(
        mut self,
        delay: &mut DelayNs,
        config: Config,
    ) -> Result<Max7456Blocking<SPI, Configured>, InitError<Self, SPI::Error>>
    where
        DelayNs: BlockingDelayNs,
    {
        if let Err(e) = self.write_end_string() {
            return Err(self.init_spi_error(e));
        }

        let osdm = match self.raw_read_register(registers::OSDM) {
            Ok(v) => v,
            Err(e) => return Err(self.init_spi_error(e)),
        };
        if osdm != registers::OSDM_DEFAULT {
            return Err(self.init_error(Error::NotFound { osdm }));
        }

        let device_type = match self.detect_device_type() {
            Ok(v) => v,
            Err(e) => return Err(self.init_error(e)),
        };

        if let Err(e) = self.raw_write_register(registers::VM0, registers::VM0_RESET) {
            return Err(self.init_spi_error(e));
        }
        delay.delay_us(config.reset_settle_delay_us);
        for _ in 0..config.reset_poll_max_tries {
            let vm0 = match self.raw_read_register(registers::VM0) {
                Ok(v) => v,
                Err(e) => return Err(self.init_spi_error(e)),
            };
            if (vm0 & registers::VM0_RESET) == 0 {
                let state = match self.apply_config(device_type, config) {
                    Ok(v) => v,
                    Err(e) => return Err(self.init_error(e)),
                };
                return Ok(Max7456Blocking {
                    bus: self.bus,
                    state: Configured { runtime: state },
                });
            }
        }
        Err(self.init_error(Error::ResetTimeout))
    }

    fn apply_config(
        &mut self,
        device_type: DeviceType,
        config: Config,
    ) -> Result<RuntimeState, Error<SPI::Error>> {
        validate_config(config)?;

        let video_mode = resolve_video_mode_blocking(self, config.video_mode)?;
        let state = runtime_state_from_config(device_type, config, video_mode)?;
        self.raw_write_register(registers::VM0, state.vm0)?;
        self.raw_write_register(registers::HOS, hos_from_offset(config.h_offset))?;
        self.raw_write_register(registers::VOS, vos_from_offset(config.v_offset))?;

        self.raw_write_register(registers::VM1, state.vm1)?;
        self.raw_write_register(registers::DMM, state.dmm)?;

        let rb = brightness_register(config.black_level, config.white_level)?;
        for register in registers::RB0..=registers::RB15 {
            self.raw_write_register(register, rb)?;
        }
        Ok(state)
    }
}

impl<SPI> Max7456Blocking<SPI, Configured>
where
    SPI: BlockingSpiDevice,
{
    pub fn set_invert(&mut self, invert: bool) -> Result<(), Error<SPI::Error>> {
        let mut state = self.state.runtime;
        state.dmm = toggled_dmm(state.dmm, invert);
        self.raw_write_register(registers::DMM, state.dmm)?;
        self.state.runtime = state;
        Ok(())
    }

    pub fn clear_display(&mut self) -> Result<(), Error<SPI::Error>> {
        let dmm = self.state.runtime.dmm;
        self.raw_write_register(registers::DMM, dmm | registers::DMM_CLEAR_DISPLAY)?;
        self.restore_dmm()
    }

    pub fn write_display_byte(
        &mut self,
        position: DisplayPosition,
        value: u8,
    ) -> Result<(), Error<SPI::Error>> {
        self.set_display_address(position.index())?;
        self.raw_write_register(registers::DMDI, sanitize_display_byte(value))?;
        Ok(())
    }

    pub fn write_display_run_at(
        &mut self,
        position: DisplayPosition,
        values: &[u8],
    ) -> Result<(), Error<SPI::Error>> {
        let len = display_slice_len(values)?;
        self.check_display_range(position.index(), len)?;
        if values.is_empty() {
            return Ok(());
        }
        if let [value] = values {
            return self.write_display_byte(position, *value);
        }

        self.write_dmm_mode(self.state.runtime.dmm | registers::DMM_AUTO_INCREMENT)?;
        self.set_display_address(position.index())?;
        for value in values {
            self.raw_write_register(registers::DMDI, sanitize_display_byte(*value))?;
        }
        self.write_end_string()?;
        self.restore_dmm()
    }

    pub fn write_char(&mut self, x: u8, y: u8, value: u8) -> Result<(), Error<SPI::Error>> {
        let position = self.display_position(x, y)?;
        self.write_display_byte(position, value)
    }

    pub fn write_text(&mut self, x: u8, y: u8, values: &[u8]) -> Result<(), Error<SPI::Error>> {
        let position = write_text_position(self.state.runtime.geometry, x, y, values)?;
        self.write_display_run_at(position, values)
    }

    pub fn clear_row(&mut self, y: u8) -> Result<(), Error<SPI::Error>> {
        let state = self.state.runtime;
        let position = display_position(state.geometry, 0, y)?;
        let spaces = [b' '; 30];
        self.write_display_run_at(position, &spaces)
    }

    fn set_display_address(&mut self, position: u16) -> Result<(), SPI::Error> {
        self.raw_write_register(registers::DMAH, (position >> 8) as u8)?;
        self.raw_write_register(registers::DMAL, position as u8)
    }

    fn write_dmm_mode(&mut self, value: u8) -> Result<(), SPI::Error> {
        self.raw_write_register(registers::DMM, value)
    }

    fn restore_dmm(&mut self) -> Result<(), Error<SPI::Error>> {
        let dmm = self.state.runtime.dmm;
        self.write_dmm_mode(dmm)?;
        Ok(())
    }

    fn display_position(&self, x: u8, y: u8) -> Result<DisplayPosition, Error<SPI::Error>> {
        display_position(self.state.runtime.geometry, x, y)
    }

    fn check_display_range(&self, position: u16, len: u16) -> Result<(), Error<SPI::Error>> {
        check_display_range(self.state.runtime.geometry, position, len)
    }
}

fn validate_config<SpiError>(config: Config) -> Result<(), Error<SpiError>> {
    if config.black_level > 3 {
        return Err(Error::Config(ConfigError::out_of_range(
            ConfigField::BlackLevel,
            0,
            3,
            config.black_level as i16,
        )));
    }
    if config.white_level > 3 {
        return Err(Error::Config(ConfigError::out_of_range(
            ConfigField::WhiteLevel,
            0,
            3,
            config.white_level as i16,
        )));
    }
    if config.background_gray > 7 {
        return Err(Error::Config(ConfigError::out_of_range(
            ConfigField::BackgroundGray,
            0,
            7,
            config.background_gray as i16,
        )));
    }
    if !(-32..=31).contains(&config.h_offset) {
        return Err(Error::Config(ConfigError::out_of_range(
            ConfigField::HOffset,
            -32,
            31,
            config.h_offset as i16,
        )));
    }
    if !(-16..=15).contains(&config.v_offset) {
        return Err(Error::Config(ConfigError::out_of_range(
            ConfigField::VOffset,
            -16,
            15,
            config.v_offset as i16,
        )));
    }
    Ok(())
}

fn vm0_value(config: Config, video_mode: VideoMode) -> u8 {
    let mut vm0 = match config.sync_source {
        SyncSource::Auto => 0,
        SyncSource::External => registers::VM0_SYNC_MODE_EXTERNAL,
        SyncSource::Internal => registers::VM0_SYNC_MODE_INTERNAL,
    };
    if config.osd_enabled {
        vm0 |= registers::VM0_OSD_ENABLE;
    }
    vm0 |= match video_mode {
        VideoMode::Pal => registers::VM0_VIDEO_MODE_PAL,
        VideoMode::Ntsc | VideoMode::Auto => registers::VM0_VIDEO_MODE_NTSC,
    };
    vm0
}

fn vm1_value(config: Config) -> u8 {
    let mut vm1 = registers::VM1_BLINK_TIME_1 | registers::VM1_BLINK_DUTY_75_25;
    if matches!(config.background_mode, BackgroundMode::Gray) {
        vm1 |= registers::VM1_BACKGROUND_MODE_GRAY;
    }
    vm1 | ((config.background_gray & 0x07) << 4)
}

fn dmm_value(config: Config) -> u8 {
    if config.invert {
        registers::DMM_INVERT_PIXEL_COLOR
    } else {
        0
    }
}

fn runtime_state_from_config<SpiError>(
    device_type: DeviceType,
    config: Config,
    video_mode: VideoMode,
) -> Result<RuntimeState, Error<SpiError>> {
    let geometry = video_mode
        .geometry()
        .ok_or(Error::Config(ConfigError::unsupported(
            ConfigField::VideoMode,
        )))?;
    Ok(RuntimeState {
        device_type,
        video_mode,
        geometry,
        vm0: vm0_value(config, video_mode),
        vm1: vm1_value(config),
        dmm: dmm_value(config),
    })
}

fn toggled_dmm(dmm: u8, invert: bool) -> u8 {
    if invert {
        dmm | registers::DMM_INVERT_PIXEL_COLOR
    } else {
        dmm & !registers::DMM_INVERT_PIXEL_COLOR
    }
}

fn row_end(geometry: ScreenGeometry, y: u8) -> u16 {
    (y as u16 + 1) * geometry.columns as u16
}

fn write_text_position<SpiError>(
    geometry: ScreenGeometry,
    x: u8,
    y: u8,
    values: &[u8],
) -> Result<DisplayPosition, Error<SpiError>> {
    let position = display_position(geometry, x, y)?;
    let len = display_slice_len(values)?;
    let row_end = row_end(geometry, y);
    let Some(end) = position.index().checked_add(len) else {
        return Err(Error::OutOfBounds {
            position: position.index(),
            len: row_end,
        });
    };
    if end > row_end {
        return Err(Error::OutOfBounds {
            position: position.index(),
            len: row_end,
        });
    }
    Ok(position)
}

fn check_display_range<SpiError>(
    geometry: ScreenGeometry,
    position: u16,
    len: u16,
) -> Result<(), Error<SpiError>> {
    let Some(end) = position.checked_add(len) else {
        return Err(Error::OutOfBounds {
            position,
            len: geometry.len,
        });
    };
    if end > geometry.len {
        return Err(Error::OutOfBounds {
            position,
            len: geometry.len,
        });
    }
    Ok(())
}

fn brightness_register<SpiError>(black: u8, white: u8) -> Result<u8, Error<SpiError>> {
    if black > 3 || white > 3 {
        let (field, got) = if black > 3 {
            (ConfigField::BlackLevel, black as i16)
        } else {
            (ConfigField::WhiteLevel, white as i16)
        };
        return Err(Error::Config(ConfigError::out_of_range(field, 0, 3, got)));
    }
    Ok((black << 2) | (3 - white))
}

fn hos_from_offset(offset: i8) -> u8 {
    (32 - offset as i16) as u8
}

fn vos_from_offset(offset: i8) -> u8 {
    (16 - offset as i16) as u8
}

fn sanitize_display_byte(value: u8) -> u8 {
    if value == registers::END_STRING {
        b' '
    } else {
        value
    }
}

fn display_slice_len<SpiError>(values: &[u8]) -> Result<u16, Error<SpiError>> {
    u16::try_from(values.len()).map_err(|_err| Error::InputTooLong {
        len: values.len(),
        max: u16::MAX,
    })
}

fn display_position<SpiError>(
    geometry: ScreenGeometry,
    x: u8,
    y: u8,
) -> Result<DisplayPosition, Error<SpiError>> {
    match geometry.index(x, y) {
        Some(index) => Ok(DisplayPosition { index }),
        None => Err(Error::InvalidCoordinate {
            x,
            y,
            columns: geometry.columns,
            rows: geometry.rows,
        }),
    }
}

fn detect_video_mode_from_stat(stat: u8) -> VideoMode {
    let los = (stat & registers::STAT_LOS) != 0;
    let pal = (stat & registers::STAT_PAL) != 0;
    if !los && pal {
        VideoMode::Pal
    } else if !los && !pal {
        // BF-compatible alt behavior: !LOS && !PAL => NTSC.
        VideoMode::Ntsc
    } else {
        // No valid signal, fallback to PAL like BF.
        VideoMode::Pal
    }
}

fn resolve_video_mode_blocking<SPI, State>(
    max7456: &mut Max7456Blocking<SPI, State>,
    requested: VideoMode,
) -> Result<VideoMode, Error<SPI::Error>>
where
    SPI: BlockingSpiDevice,
{
    match requested {
        VideoMode::Pal | VideoMode::Ntsc => Ok(requested),
        VideoMode::Auto => {
            let stat = max7456.raw_read_register(registers::STAT)?;
            Ok(detect_video_mode_from_stat(stat))
        }
    }
}

async fn resolve_video_mode_async<SPI, State>(
    max7456: &mut Max7456Async<SPI, State>,
    requested: VideoMode,
) -> Result<VideoMode, Error<SPI::Error>>
where
    SPI: AsyncSpiDevice,
{
    match requested {
        VideoMode::Pal | VideoMode::Ntsc => Ok(requested),
        VideoMode::Auto => {
            let stat = max7456.raw_read_register(registers::STAT).await?;
            Ok(detect_video_mode_from_stat(stat))
        }
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
    use std::vec::Vec;

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
        let expectations = init_success_expectations(registers::CMAL_CA8_BIT);
        let spi = SpiMock::new(&expectations);
        let driver = Max7456Blocking::new(spi);
        let mut delay = NoopDelay;
        let driver = driver.init(&mut delay, test_config()).unwrap();
        let state = driver.state();
        assert_eq!(state.device_type, DeviceType::At7456e);
        assert_eq!(state.video_mode, VideoMode::Pal);
        assert_eq!(
            state.geometry,
            ScreenGeometry {
                columns: 30,
                rows: 16,
                len: 480
            }
        );
        let mut spi = driver.release();
        spi.done();
    }

    #[futures_test::test]
    async fn test_init_at7456e_async() {
        let expectations = init_success_expectations(registers::CMAL_CA8_BIT);
        let spi = SpiMock::new(&expectations);
        let driver = Max7456Async::new(spi);
        let mut delay = AsyncNoopDelay;
        let driver = driver.init(&mut delay, test_config()).await.unwrap();
        let state = driver.state();
        assert_eq!(state.device_type, DeviceType::At7456e);
        assert_eq!(state.geometry.len, 480);
        let mut spi = driver.release();
        spi.done();
    }

    #[test]
    fn test_init_max7456_blocking() {
        let expectations = init_success_expectations(0);
        let spi = SpiMock::new(&expectations);
        let driver = Max7456Blocking::new(spi);
        let mut delay = NoopDelay;
        let driver = driver.init(&mut delay, test_config()).unwrap();
        let state = driver.state();
        assert_eq!(state.device_type, DeviceType::Max7456);
        let mut spi = driver.release();
        spi.done();
    }

    #[test]
    fn test_init_auto_video_detects_ntsc_geometry() {
        let config = Config {
            video_mode: VideoMode::Auto,
            ..Config::default()
        };
        let expectations =
            init_success_expectations_with_config(registers::CMAL_CA8_BIT, config, Some(0));
        let spi = SpiMock::new(&expectations);
        let driver = Max7456Blocking::new(spi);
        let mut delay = NoopDelay;
        let driver = driver.init(&mut delay, config).unwrap();
        let state = driver.state();
        assert_eq!(state.video_mode, VideoMode::Ntsc);
        assert_eq!(state.geometry.rows, 13);
        assert_eq!(state.geometry.len, 390);
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
        let driver = Max7456Blocking::new(spi);
        let mut delay = NoopDelay;
        let init_err = driver.init(&mut delay, test_config()).err().unwrap();
        assert!(matches!(init_err.error, Error::NotFound { osdm: 0x00 }));
        let mut spi = init_err.driver.release();
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
        let driver = Max7456Async::new(spi);
        let mut delay = AsyncNoopDelay;
        let init_err = driver.init(&mut delay, test_config()).await.err().unwrap();
        assert!(matches!(init_err.error, Error::NotFound { osdm: 0x00 }));
        let mut spi = init_err.driver.release();
        spi.done();
    }

    #[test]
    fn test_init_nvr_busy_timeout_blocking() {
        let mut expectations = vec![
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::END_STRING]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::OSDM | registers::READ]),
            SpiTransaction::read(registers::OSDM_DEFAULT),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::VM0, 0]),
            SpiTransaction::transaction_end(),
        ];
        for _ in 0..NVR_BUSY_POLL_TRIES {
            push_read(&mut expectations, registers::STAT, registers::STAT_NVR_BUSY);
        }

        let spi = SpiMock::new(&expectations);
        let driver = Max7456Blocking::new(spi);
        let mut delay = NoopDelay;
        let init_err = driver.init(&mut delay, test_config()).err().unwrap();
        assert!(matches!(init_err.error, Error::NvrBusyTimeout));
        let mut spi = init_err.driver.release();
        spi.done();
    }

    #[test]
    fn test_display_byte_writes_address_and_sanitized_data() {
        let mut expectations = init_success_expectations(registers::CMAL_CA8_BIT);
        push_write(&mut expectations, registers::DMAH, 0x01);
        push_write(&mut expectations, registers::DMAL, 0xdf);
        push_write(&mut expectations, registers::DMDI, b' ');

        let spi = SpiMock::new(&expectations);
        let driver = Max7456Blocking::new(spi);
        let mut delay = NoopDelay;
        let mut driver = driver.init(&mut delay, test_config()).unwrap();
        driver.write_char(29, 15, registers::END_STRING).unwrap();
        let mut spi = driver.release();
        spi.done();
    }

    #[test]
    fn test_display_run_uses_auto_increment_and_restores_dmm() {
        let config = Config {
            invert: true,
            ..test_config()
        };
        let mut expectations =
            init_success_expectations_with_config(registers::CMAL_CA8_BIT, config, None);
        push_write(
            &mut expectations,
            registers::DMM,
            registers::DMM_INVERT_PIXEL_COLOR | registers::DMM_AUTO_INCREMENT,
        );
        push_write(&mut expectations, registers::DMAH, 0x00);
        push_write(&mut expectations, registers::DMAL, 0x00);
        push_write(&mut expectations, registers::DMDI, b'A');
        push_write(&mut expectations, registers::DMDI, b' ');
        push_write(&mut expectations, registers::DMDI, b'C');
        expectations.push(SpiTransaction::transaction_start());
        expectations.push(SpiTransaction::write_vec(vec![registers::END_STRING]));
        expectations.push(SpiTransaction::transaction_end());
        push_write(
            &mut expectations,
            registers::DMM,
            registers::DMM_INVERT_PIXEL_COLOR,
        );

        let spi = SpiMock::new(&expectations);
        let driver = Max7456Blocking::new(spi);
        let mut delay = NoopDelay;
        let mut driver = driver.init(&mut delay, config).unwrap();
        driver
            .write_text(0, 0, &[b'A', registers::END_STRING, b'C'])
            .unwrap();
        assert_eq!(driver.state().dmm, registers::DMM_INVERT_PIXEL_COLOR);
        let mut spi = driver.release();
        spi.done();
    }

    #[test]
    fn test_display_write_bounds_after_init() {
        let expectations = init_success_expectations(registers::CMAL_CA8_BIT);
        let spi = SpiMock::new(&expectations);
        let driver = Max7456Blocking::new(spi);
        let mut delay = NoopDelay;
        let mut driver = driver.init(&mut delay, test_config()).unwrap();
        let err = driver.write_char(30, 0, b'A').err().unwrap();
        assert!(matches!(
            err,
            Error::InvalidCoordinate {
                x: 30,
                y: 0,
                columns: 30,
                rows: 16
            }
        ));
        let mut spi = driver.release();
        spi.done();
    }

    fn test_config() -> Config {
        Config {
            video_mode: VideoMode::Pal,
            ..Config::default()
        }
    }

    fn push_write(expectations: &mut Vec<SpiTransaction<u8>>, register: u8, value: u8) {
        expectations.push(SpiTransaction::transaction_start());
        expectations.push(SpiTransaction::write_vec(vec![register, value]));
        expectations.push(SpiTransaction::transaction_end());
    }

    fn push_read(expectations: &mut Vec<SpiTransaction<u8>>, register: u8, value: u8) {
        expectations.push(SpiTransaction::transaction_start());
        expectations.push(SpiTransaction::write_vec(vec![register | registers::READ]));
        expectations.push(SpiTransaction::read(value));
        expectations.push(SpiTransaction::transaction_end());
    }

    fn init_success_expectations(cmal_readback: u8) -> Vec<SpiTransaction<u8>> {
        init_success_expectations_with_config(cmal_readback, test_config(), None)
    }

    fn init_success_expectations_with_config(
        cmal_readback: u8,
        config: Config,
        auto_video_stat: Option<u8>,
    ) -> Vec<SpiTransaction<u8>> {
        let mut expectations: Vec<SpiTransaction<u8>> = vec![
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::END_STRING]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::OSDM | registers::READ]),
            SpiTransaction::read(0x1b),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::VM0, 0]),
            SpiTransaction::transaction_end(),
        ];

        push_read(&mut expectations, registers::STAT, 0x00);
        let detect_tries = if (cmal_readback & registers::CMAL_CA8_BIT) != 0 {
            1
        } else {
            3
        };
        for _ in 0..detect_tries {
            push_write(&mut expectations, registers::CMAL, registers::CMAL_CA8_BIT);
            push_read(&mut expectations, registers::CMAL, cmal_readback);
        }

        expectations.extend([
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::VM0, registers::VM0_RESET]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![registers::VM0 | registers::READ]),
            SpiTransaction::read(0x00),
            SpiTransaction::transaction_end(),
        ]);

        if let Some(stat) = auto_video_stat {
            push_read(&mut expectations, registers::STAT, stat);
        }

        let resolved_mode = if let Some(stat) = auto_video_stat {
            detect_video_mode_from_stat(stat)
        } else {
            config.video_mode
        };
        push_write(
            &mut expectations,
            registers::VM0,
            vm0_value(config, resolved_mode),
        );
        push_write(
            &mut expectations,
            registers::HOS,
            hos_from_offset(config.h_offset),
        );
        push_write(
            &mut expectations,
            registers::VOS,
            vos_from_offset(config.v_offset),
        );
        push_write(&mut expectations, registers::VM1, vm1_value(config));
        push_write(&mut expectations, registers::DMM, dmm_value(config));

        for register in registers::RB0..=registers::RB15 {
            let rb = brightness_register::<()>(config.black_level, config.white_level).unwrap();
            push_write(&mut expectations, register, rb);
        }

        expectations
    }
}
