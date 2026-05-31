#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    pub reset_settle_delay_us: u32,
    pub reset_poll_max_tries: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Matches Betaflight behavior.
            reset_settle_delay_us: 200,
            reset_poll_max_tries: 1024,
        }
    }
}
