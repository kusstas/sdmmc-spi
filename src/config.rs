/// Represents config for [`SdMmcSpi`](crate::SdMmcSpi).
pub trait SdMmcSpiConfig {
    /// Max attempts to send command.
    const CMD_MAX_ATTEMPTS: usize;
    /// Max attempts to read R1.
    const READ_R1_ATTEMPTS: usize;
    /// Max attempts to enter SPI mode.
    const ENTER_SPI_MODE_ATTEMPTS: usize;
    /// Count of dummy cycles for delay.
    const DELAY_DUMMY_CYCLES: usize;
}

/// Default implementation of [`SdMmcSpiConfig`](crate::SdMmcSpiConfig).
pub struct DefaultSdMmcSpiConfig;

impl SdMmcSpiConfig for DefaultSdMmcSpiConfig {
    const CMD_MAX_ATTEMPTS: usize = 256;
    const READ_R1_ATTEMPTS: usize = 128;
    const ENTER_SPI_MODE_ATTEMPTS: usize = 10;
    const DELAY_DUMMY_CYCLES: usize = 32;
}
