use crate::error::ConfigError;
use crate::error::ConfigField;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum VideoMode {
    Pal,
    Ntsc,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ScreenGeometry {
    pub columns: u8,
    pub rows: u8,
    pub len: u16,
}

impl VideoMode {
    pub const fn geometry(self) -> Option<ScreenGeometry> {
        match self {
            Self::Pal => Some(ScreenGeometry {
                columns: 30,
                rows: 16,
                len: 480,
            }),
            Self::Ntsc => Some(ScreenGeometry {
                columns: 30,
                rows: 13,
                len: 390,
            }),
            Self::Auto => None,
        }
    }
}

impl ScreenGeometry {
    pub const fn cell_count(self) -> u16 {
        self.len
    }

    pub const fn contains(self, x: u8, y: u8) -> bool {
        x < self.columns && y < self.rows
    }

    pub const fn index(self, x: u8, y: u8) -> Option<u16> {
        if self.contains(x, y) {
            Some((y as u16 * self.columns as u16) + x as u16)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SyncSource {
    Auto,
    Internal,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BackgroundMode {
    Transparent,
    Gray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    pub reset_settle_delay_us: u32,
    pub reset_poll_max_tries: u16,
    pub video_mode: VideoMode,
    pub sync_source: SyncSource,
    pub h_offset: i8,
    pub v_offset: i8,
    pub osd_enabled: bool,
    pub invert: bool,
    pub black_level: u8,
    pub white_level: u8,
    pub background_mode: BackgroundMode,
    pub background_gray: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Matches Betaflight behavior.
            reset_settle_delay_us: 200,
            reset_poll_max_tries: 1024,
            video_mode: VideoMode::Auto,
            sync_source: SyncSource::Auto,
            h_offset: 0,
            v_offset: 0,
            osd_enabled: true,
            invert: false,
            black_level: 0,
            white_level: 2,
            background_mode: BackgroundMode::Transparent,
            // 28% brightness (chip default in BF).
            background_gray: 4,
        }
    }
}

impl Config {
    pub fn auto() -> Self {
        Self {
            video_mode: VideoMode::Auto,
            ..Self::default()
        }
    }

    pub fn pal() -> Self {
        Self {
            video_mode: VideoMode::Pal,
            ..Self::default()
        }
    }

    pub fn ntsc() -> Self {
        Self {
            video_mode: VideoMode::Ntsc,
            ..Self::default()
        }
    }

    pub fn with_sync_source(mut self, sync_source: SyncSource) -> Self {
        self.sync_source = sync_source;
        self
    }

    pub fn with_osd_enabled(mut self, enabled: bool) -> Self {
        self.osd_enabled = enabled;
        self
    }

    pub fn with_invert(mut self, invert: bool) -> Self {
        self.invert = invert;
        self
    }

    pub fn with_transparent_background(mut self) -> Self {
        self.background_mode = BackgroundMode::Transparent;
        self
    }

    pub fn with_gray_background(mut self, gray: u8) -> Result<Self, ConfigError> {
        if gray > 7 {
            return Err(ConfigError::out_of_range(
                ConfigField::BackgroundGray,
                0,
                7,
                gray as i16,
            ));
        }
        self.background_mode = BackgroundMode::Gray;
        self.background_gray = gray;
        Ok(self)
    }

    pub fn with_offsets(mut self, h_offset: i8, v_offset: i8) -> Result<Self, ConfigError> {
        if !(-32..=31).contains(&h_offset) {
            return Err(ConfigError::out_of_range(
                ConfigField::HOffset,
                -32,
                31,
                h_offset as i16,
            ));
        }
        if !(-16..=15).contains(&v_offset) {
            return Err(ConfigError::out_of_range(
                ConfigField::VOffset,
                -16,
                15,
                v_offset as i16,
            ));
        }
        self.h_offset = h_offset;
        self.v_offset = v_offset;
        Ok(self)
    }

    pub fn with_brightness(mut self, black: u8, white: u8) -> Result<Self, ConfigError> {
        if black > 3 {
            return Err(ConfigError::out_of_range(
                ConfigField::BlackLevel,
                0,
                3,
                black as i16,
            ));
        }
        if white > 3 {
            return Err(ConfigError::out_of_range(
                ConfigField::WhiteLevel,
                0,
                3,
                white as i16,
            ));
        }
        self.black_level = black;
        self.white_level = white;
        Ok(self)
    }
}
