//! SD/MMC Library written in Embedded Rust, that inspired by [embedded-sdmmc](https://crates.io/crates/embedded-sdmmc).
//!
//! This crate is intended to allow you to init/read/write SD/MMC devices by SPI bus.

#![no_std]

mod config;
mod consts;
mod crc;
mod csd;
mod response;

pub use crate::config::{DefaultSdMmcSpiConfig, SdMmcSpiConfig};
pub use diskio::{
    BlockSize, DiskioDevice, Error as DiskioError, IoctlCmd, Lba, Status, StatusFlag,
};

use crate::{
    consts::{commands, tokens, BLOCK_SIZE},
    crc::{crc16, crc7},
    csd::{CapacityProvider, Csd, CsdData, CsdV1, CsdV2},
    response::R1Response,
};

use core::{cell::RefCell, marker::PhantomData};
use defmt::{error, info, warn, Format};
use embedded_hal::blocking::spi::Transfer;
use switch_hal::OutputSwitch;

/// [`SdMmcSpi`] result error.
///
/// `T` - transport error type.
/// `S` - select switch type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<T, S> {
    /// Error from the SPI peripheral.
    Transport(T),
    /// Couldn't set a select.
    SelectError(S),
    /// Failed to enable CRC checking on the card.
    CantEnableCRC,
    /// No response when reading data from the card.
    TimeoutReadBuffer,
    /// No response when waiting for the card to not be busy.
    TimeoutWaitAvailable,
    /// No response when executing this command.
    TimeoutCommand(u8),
    /// Command error.
    ErrorCommand(u8),
    /// Failed to read the Card Specific Data register.
    RegisterReadError,
    /// CRC mismatch (card, host).
    CrcError(u16, u16),
    /// Error reading from the card.
    ReadError,
    /// Error writing to the card.
    WriteError,
    /// Can't perform this operation with the card in this state.
    BadState,
    /// Couldn't find the card.
    CardNotFound,
}

/// Card type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum CardType {
    SD1,
    SD2,
    SDHC,
}

/// Error type alias.
type ErrorFor<T> = <T as DiskioDevice>::HardwareError;

/// SD Card SPI driver.
///
/// `Spi` - SPI.
/// `Cs` - Chip select output switch.
/// `Config` - Config implementation of driver config trait.
pub struct SdMmcSpi<Spi: Transfer<u8>, Cs: OutputSwitch, Config: SdMmcSpiConfig> {
    spi: RefCell<Spi>,
    cs: RefCell<Cs>,
    status: Status,
    card_type: CardType,
    csd: Csd,
    config: PhantomData<Config>,
}

impl<Spi: Transfer<u8>, Cs: OutputSwitch, Config: SdMmcSpiConfig> SdMmcSpi<Spi, Cs, Config>
where
    Spi::Error: core::fmt::Debug,
    Cs::Error: core::fmt::Debug,
{
    /// Init sequence value.
    const INIT_SET_VALUE: u8 = 0xFF;
    /// Init sequence size.
    const INIT_SET_SIZE: usize = 10;
    /// Receive transfer token.
    const RECEIVE_TRANSFER_TOKEN: u8 = 0xFF;

    /// Creates a new [`SdMmcSpi<Spi, Cs, Config>`].
    ///
    /// `spi` - SPI instance.
    /// `cs` - chip select output switch.
    pub fn new(spi: Spi, cs: Cs) -> Self {
        SdMmcSpi {
            spi: RefCell::new(spi),
            cs: RefCell::new(cs),
            status: StatusFlag::NotInitialized.into(),
            card_type: CardType::SD1,
            csd: Csd::V1(CsdV1(0)),
            config: PhantomData::<Config>,
        }
    }

    /// Validate buffer for read/write.
    fn validate_buffer_len(buf_len: usize) -> Result<(), DiskioError<ErrorFor<Self>>> {
        if buf_len == 0 || buf_len % BLOCK_SIZE != 0 {
            error!(
                "SD invalid buffer, length: {}, block size: {}",
                buf_len, BLOCK_SIZE
            );
            Err(DiskioError::InvalidArgument)
        } else {
            Ok(())
        }
    }

    /// Validate initialzed.
    fn validate_initialized(&self) -> Result<(), DiskioError<ErrorFor<Self>>> {
        if self.status.contains(StatusFlag::NotInitialized) {
            Err(DiskioError::NotInitialized)
        } else {
            Ok(())
        }
    }

    /// Get count of blocks in buffer.
    fn get_block_count(buf_len: usize) -> usize {
        buf_len / BLOCK_SIZE
    }

    /// Delay.
    fn delay() {
        for i in 0..Config::DELAY_DUMMY_CYCLES {
            unsafe { core::ptr::read_volatile(&i) };
        }
    }

    /// Convert lba.
    fn convert_lba(&self, lba: Lba) -> u32 {
        match self.card_type {
            CardType::SD1 | CardType::SD2 => (lba as usize * BLOCK_SIZE) as u32,
            CardType::SDHC => lba as u32,
        }
    }

    /// Activate chip select.
    fn select(&self) -> Result<(), ErrorFor<Self>> {
        self.cs.borrow_mut().on().map_err(Error::SelectError)
    }

    /// Deactivate chip select.
    fn unselect(&self) -> Result<(), ErrorFor<Self>> {
        self.cs.borrow_mut().off().map_err(Error::SelectError)
    }

    /// CS scope.
    fn cs_scope<F>(&self, f: F) -> Result<(), ErrorFor<Self>>
    where
        F: FnOnce(&Self) -> Result<(), ErrorFor<Self>>,
    {
        self.select()?;
        let result = f(self);
        self.unselect()?;

        result
    }

    /// CS scope mut.
    fn cs_scope_mut<F>(&mut self, f: F) -> Result<(), ErrorFor<Self>>
    where
        F: FnOnce(&mut Self) -> Result<(), ErrorFor<Self>>,
    {
        self.select()?;
        let result = f(self);
        self.unselect()?;

        result
    }

    /// Send one byte and receive one byte.
    fn transfer(&self, data: u8) -> Result<u8, ErrorFor<Self>> {
        self.spi
            .borrow_mut()
            .transfer(&mut [data])
            .map(|b| b[0])
            .map_err(Error::Transport)
    }

    /// Receive a byte from the SD card by clocking in an 0xFF byte.
    fn receive(&self) -> Result<u8, ErrorFor<Self>> {
        self.transfer(Self::RECEIVE_TRANSFER_TOKEN)
    }

    /// Send a byte to the SD card.
    fn send(&self, data: u8) -> Result<(), ErrorFor<Self>> {
        self.transfer(data).map(|_| ())
    }

    /// Receive a slice from the SD card.
    fn receive_slice(&self, data: &mut [u8]) -> Result<(), ErrorFor<Self>> {
        for byte in data.iter_mut() {
            *byte = self.receive()?;
        }

        Ok(())
    }

    /// Send a slice to the SD card.
    fn send_slice(&self, data: &[u8]) -> Result<(), ErrorFor<Self>> {
        for byte in data.iter() {
            self.send(*byte)?;
        }

        Ok(())
    }

    /// Skip byte.
    fn skip_byte(&self) -> Result<(), ErrorFor<Self>> {
        self.receive().map(|_| ())
    }

    /// Wait for token.
    fn wait_for_token<F: Fn(u8) -> bool>(
        &self,
        token_validator: F,
        error: ErrorFor<Self>,
    ) -> Result<u8, ErrorFor<Self>> {
        for _ in 0..Config::CMD_MAX_ATTEMPTS {
            let token = self.receive()?;

            if token_validator(token) {
                return Ok(token);
            }

            Self::delay();
        }

        Err(error)
    }

    /// Wait available state of card.
    fn wait_available_state(&self) -> Result<(), ErrorFor<Self>> {
        self.wait_for_token(
            |token| token == tokens::AVAILABLE,
            Error::TimeoutWaitAvailable,
        )
        .map(|_| ())
    }

    /// Send command implementation.
    fn send_command_impl(&self, cmd: u8, arg: u32) -> Result<R1Response, ErrorFor<Self>> {
        self.wait_available_state()?;

        let mut buf = [
            cmd,
            (arg >> 24) as u8,
            (arg >> 16) as u8,
            (arg >> 8) as u8,
            arg as u8,
            0,
        ];
        let crc_index = buf.len() - 1;

        buf[crc_index] = (crc7(&buf[..crc_index]) << 1) | 0x01;

        self.send_slice(&buf)?;

        if cmd == commands::CMD12 {
            self.skip_byte()?;
        }

        for _ in 0..Config::READ_R1_ATTEMPTS {
            let r1 = R1Response(self.receive()?);

            if r1.is_valid() {
                return Ok(r1);
            }
        }

        Err(Error::TimeoutCommand(cmd))
    }

    /// Send command.
    fn send_command(&self, cmd: u8, arg: u32) -> Result<R1Response, ErrorFor<Self>> {
        if (cmd & commands::ACMD_FLAG) != 0 {
            self.send_command_impl(commands::CMD55, 0x0000_0000)?;
        }

        self.send_command_impl(cmd & !commands::ACMD_FLAG, arg)
    }

    /// Read data.
    fn read_data(&self, data: &mut [u8]) -> Result<(), ErrorFor<Self>> {
        if self.wait_for_token(|token| token != tokens::AVAILABLE, Error::TimeoutReadBuffer)?
            != tokens::DATA_START_BLOCK
        {
            return Err(Error::ReadError);
        }

        self.receive_slice(data)?;

        let card_crc = (u16::from(self.receive()?) << 8) | u16::from(self.receive()?);
        let host_crc = crc16(data);

        if card_crc != host_crc {
            return Err(Error::CrcError(card_crc, host_crc));
        }

        Ok(())
    }

    /// Write data.
    fn write_data(&self, token: u8, data: &[u8]) -> Result<(), ErrorFor<Self>> {
        let host_crc = crc16(data);

        self.send(token)?;
        self.send_slice(data)?;
        self.send((host_crc >> 8) as u8)?;
        self.send(host_crc as u8)?;

        if (self.receive()? & tokens::DATA_RES_MASK) != tokens::DATA_RES_ACCEPTED {
            Err(Error::WriteError)
        } else {
            Ok(())
        }
    }

    /// Enter SD to SPI mode.
    fn enter_spi_mode(&self) -> Result<(), ErrorFor<Self>> {
        for i in 0..Config::ENTER_SPI_MODE_ATTEMPTS {
            info!("Enter to SPI mode for SD, attempt: {}", i + 1);

            match self.send_command(commands::CMD0, 0x0000_0000) {
                Ok(R1Response::IN_IDLE_STATE) => return Ok(()),
                Ok(r) => warn!(
                    "Wrong response from CMD{}: 0b{:02X}",
                    commands::CMD0 - commands::CMD_BASE,
                    r.0
                ),
                Err(Error::TimeoutCommand(commands::CMD0)) => {}
                Err(err) => return Err(err),
            }

            Self::delay();
        }

        Err(Error::TimeoutCommand(commands::CMD0))
    }

    /// Enable CRC.
    fn enable_crc(&self) -> Result<(), ErrorFor<Self>> {
        info!("Enabling CRC for SD");

        if self.send_command(commands::CMD59, 0x0000_0001)? != R1Response::IN_IDLE_STATE {
            Err(Error::CantEnableCRC)
        } else {
            Ok(())
        }
    }

    /// Verify SD Memory Card interface operating condition.
    fn send_if_cond(&self) -> Result<CardType, ErrorFor<Self>> {
        info!("Verifing SD Memory Card interface operating condition");

        for _ in 0..Config::CMD_MAX_ATTEMPTS {
            if self.send_command(commands::CMD8, 0x0000_01AA)? == R1Response::IN_IDLE_AND_ILLEGAL {
                return Ok(CardType::SD1);
            }

            self.skip_byte()?;
            self.skip_byte()?;
            self.skip_byte()?;

            if self.receive()? == tokens::CMD8_STATUS {
                return Ok(CardType::SD2);
            }
        }

        Err(Error::TimeoutCommand(commands::CMD8))
    }

    /// Sends host capacity support information and activates.
    fn send_op_comd(&self, arg: u32) -> Result<(), ErrorFor<Self>> {
        info!("Sending host capacity support information and activates");

        for _ in 0..Config::CMD_MAX_ATTEMPTS {
            if self.send_command(commands::ACMD41, arg)? == R1Response::READY_STATE {
                return Ok(());
            }
        }

        Err(Error::TimeoutCommand(commands::ACMD41))
    }

    /// Check SD type.
    fn check_type(&self) -> Result<CardType, ErrorFor<Self>> {
        info!("Checking SD type");

        let mut card_type = self.send_if_cond()?;

        let arg = match card_type {
            CardType::SD1 => 0x0000_0000,
            CardType::SD2 | CardType::SDHC => 0x4000_0000,
        };

        self.send_op_comd(arg)?;

        if card_type == CardType::SD2 {
            if self.send_command(commands::CMD58, 0x0000_0000)? != R1Response::READY_STATE {
                return Err(Error::ErrorCommand(commands::CMD58));
            }
            if (self.receive()? & tokens::CMD58_OCR) == tokens::CMD58_OCR {
                card_type = CardType::SDHC;
            }

            self.skip_byte()?;
            self.skip_byte()?;
            self.skip_byte()?;
        }

        Ok(card_type)
    }

    /// Read CSD.
    fn read_csd(&self) -> Result<Csd, ErrorFor<Self>> {
        let mut csd_data: CsdData = Default::default();

        if self.send_command(commands::CMD9, 0x0000_0000)? != R1Response::READY_STATE {
            return Err(Error::RegisterReadError);
        }

        self.read_data(&mut csd_data)?;

        Ok(match self.card_type {
            CardType::SD1 => Csd::V1(CsdV1::from(csd_data)),
            CardType::SD2 | CardType::SDHC => Csd::V2(CsdV2::from(csd_data)),
        })
    }

    /// Initialize SD.
    fn init(&mut self) -> Result<(), ErrorFor<Self>> {
        info!("SD initialize started");

        self.unselect()?;

        for _ in 0..Self::INIT_SET_SIZE {
            self.send(Self::INIT_SET_VALUE)?;
        }

        let mut result = self.cs_scope_mut(|s| {
            s.enter_spi_mode()?;
            s.enable_crc()?;

            s.card_type = s.check_type()?;
            s.csd = s.read_csd()?;

            Ok(())
        });

        self.status = match &result {
            Ok(_) => {
                info!(
                    "SD successfully initialized, version: {}, capacity: {}",
                    &self.card_type,
                    defmt::Debug2Format(&self.csd.card_capacity())
                );
                Status::default()
            }
            Err(err) => {
                error!("Failed to initialize SD: {}", defmt::Debug2Format(err));
                result = Err(Error::CardNotFound);
                StatusFlag::ErrorOccured | StatusFlag::NotInitialized
            }
        };

        result
    }
}

impl<Spi: Transfer<u8>, Cs: OutputSwitch, Config: SdMmcSpiConfig> DiskioDevice
    for SdMmcSpi<Spi, Cs, Config>
where
    Spi::Error: core::fmt::Debug,
    Cs::Error: core::fmt::Debug,
{
    type HardwareError = Error<Spi::Error, Cs::Error>;

    fn status(&self) -> Status {
        self.status
    }

    fn reset(&mut self) {
        info!("SD reset invoked");
        self.status = StatusFlag::NotInitialized.into();
    }

    fn initialize(&mut self) -> Result<(), DiskioError<Self::HardwareError>> {
        if !self.status.contains(StatusFlag::NotInitialized) {
            warn!("SD already is initialized");
            return Err(DiskioError::AlreadyInitialized);
        }

        self.init().map_err(DiskioError::Hardware)
    }

    fn read(&self, buf: &mut [u8], lba: Lba) -> Result<(), DiskioError<Self::HardwareError>> {
        Self::validate_buffer_len(buf.len())?;
        self.validate_initialized()?;

        let block_count = Self::get_block_count(buf.len());
        let lba = self.convert_lba(lba);

        self.cs_scope(|s| {
            if block_count == 1 {
                s.send_command(commands::CMD17, lba)?;
                s.read_data(buf)?;
            } else {
                s.send_command(commands::CMD18, lba)?;
                for chunk in buf.chunks_mut(BLOCK_SIZE) {
                    s.read_data(chunk)?;
                }
                s.send_command(commands::CMD12, 0x0000_0000)?;
            }

            Ok(())
        })
        .map_err(DiskioError::Hardware)
    }

    fn write(&self, buf: &[u8], lba: Lba) -> Result<(), DiskioError<Self::HardwareError>> {
        Self::validate_buffer_len(buf.len())?;
        self.validate_initialized()?;

        let block_count = Self::get_block_count(buf.len());
        let lba = self.convert_lba(lba);

        self.cs_scope(|s| {
            if block_count == 1 {
                s.send_command(commands::CMD24, lba)?;
                s.write_data(tokens::DATA_START_BLOCK, buf)?;
                s.wait_available_state()?;
                if s.send_command(commands::CMD13, 0x0000_0000)? != R1Response::READY_STATE {
                    return Err(Error::WriteError);
                }
                if s.receive()? != R1Response::READY_STATE.0 {
                    return Err(Error::WriteError);
                }
            } else {
                s.send_command(commands::CMD25, lba)?;
                for block in buf.chunks(BLOCK_SIZE) {
                    s.wait_available_state()?;
                    self.write_data(tokens::WRITE_MULTIPLE, block)?;
                }
                s.wait_available_state()?;
                s.send(tokens::STOP_TRAN)?;
            }

            Ok(())
        })
        .map_err(DiskioError::Hardware)
    }

    fn ioctl(&self, cmd: IoctlCmd) -> Result<(), DiskioError<Self::HardwareError>> {
        match cmd {
            IoctlCmd::CtrlSync => self.wait_available_state().map_err(DiskioError::Hardware),
            IoctlCmd::GetBlockSize(block_size) => {
                *block_size = BLOCK_SIZE;
                Ok(())
            }
            _ => Err(DiskioError::NotSupported),
        }
    }
}
