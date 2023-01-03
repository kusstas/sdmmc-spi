use crate::consts::BLOCK_SIZE_U64;

use bitfield::bitfield;
use size::{consts::KiB, Size};

/// Card Specific Data block.
pub type CsdData = [u8; 16];

bitfield! {
    /// Card Specific Data, version 1.
    pub struct CsdV1(u128);
    pub u8, version, _: 127, 126;
    pub u8, data_read_access_time1, _: 119, 112;
    pub u8, data_read_access_time2, _: 111, 104;
    pub u8, max_data_transfer_rate, _: 103, 96;
    pub u16, card_command_classes, _: 95, 84;
    pub u8, read_block_length, _: 83, 80;
    pub read_partial_blocks, _: 79;
    pub write_block_misalignment, _: 78;
    pub read_block_misalignment, _: 77;
    pub dsr_implemented, _: 76;
    pub u16, device_size, _: 73, 62;
    pub u8, max_read_current_vdd_max, _: 61, 59;
    pub u8, max_read_current_vdd_min, _: 58, 56;
    pub u8, max_write_current_vdd_max, _: 55, 53;
    pub u8, max_write_current_vdd_min, _: 52, 50;
    pub u8, device_size_multiplier, _: 49, 47;
    pub erase_single_block_enabled, _: 46;
    pub u8, erase_sector_size, _: 45, 39;
    pub u8, write_protect_group_size, _: 38, 32;
    pub write_protect_group_enable, _: 31;
    pub u8, write_speed_factor, _: 28, 26;
    pub u8, max_write_data_length, _: 25, 22;
    pub write_partial_blocks_allowed, _: 21;
    pub file_format_group, _: 15;
    pub copy_flag, _: 14;
    pub permanent_write_protection, _: 13;
    pub temporary_write_protection, _: 12;
    pub u8, file_format, _: 11, 10;
    pub u8, crc, _: 7, 1;
}

bitfield! {
    /// Card Specific Data, version 2.
    pub struct CsdV2(u128);
    pub u8, version, _: 127, 126;
    pub u8, data_read_access_time1, _: 119, 112;
    pub u8, data_read_access_time2, _: 111, 104;
    pub u8, max_data_transfer_rate, _: 103, 96;
    pub u16, card_command_classes, _: 95, 84;
    pub u8, read_block_length, _: 83, 80;
    pub read_partial_blocks, _: 79;
    pub write_block_misalignment, _: 78;
    pub read_block_misalignment, _: 77;
    pub dsr_implemented, _: 76;
    pub u32, device_size, _: 69, 48;
    pub erase_single_block_enabled, _: 46;
    pub u8, erase_sector_size, _: 45, 39;
    pub u8, write_protect_group_size, _: 38, 32;
    pub write_protect_group_enable, _: 31;
    pub u8, write_speed_factor, _: 28, 26;
    pub u8, max_write_data_length, _: 25, 22;
    pub write_partial_blocks_allowed, _: 21;
    pub file_format_group, _: 15;
    pub copy_flag, _: 14;
    pub permanent_write_protection, _: 13;
    pub temporary_write_protection, _: 12;
    pub u8, file_format, _: 11, 10;
    pub u8, crc, _: 7, 1;
}

/// Card Specific Data, generic container.
pub enum Csd {
    V1(CsdV1),
    V2(CsdV2),
}

/// Represents capacity provider.
pub trait CapacityProvider {
    /// Returns the card capacity in bytes.
    fn card_capacity(&self) -> Size;

    /// Returns the card capacity in 512-byte blocks.
    fn card_capacity_blocks(&self) -> u64;
}

impl From<CsdData> for CsdV1 {
    fn from(csd_data: CsdData) -> Self {
        CsdV1(u128::from_be_bytes(csd_data))
    }
}

impl From<CsdData> for CsdV2 {
    fn from(csd_data: CsdData) -> Self {
        CsdV2(u128::from_be_bytes(csd_data))
    }
}

impl CapacityProvider for CsdV1 {
    fn card_capacity(&self) -> Size {
        Size::from_bytes(
            self.device_size() << (self.device_size_multiplier() + self.read_block_length() + 2),
        )
    }

    fn card_capacity_blocks(&self) -> u64 {
        (u64::from(self.device_size()) + 1)
            << (self.device_size_multiplier() + self.read_block_length() - 7)
    }
}

impl CapacityProvider for CsdV2 {
    fn card_capacity(&self) -> Size {
        Size::from_bytes(self.card_capacity_blocks() * BLOCK_SIZE_U64)
    }

    fn card_capacity_blocks(&self) -> u64 {
        ((self.device_size() + 1) as u64) * (KiB as u64)
    }
}

impl CapacityProvider for Csd {
    fn card_capacity(&self) -> Size {
        match self {
            Csd::V1(csd) => csd.card_capacity(),
            Csd::V2(csd) => csd.card_capacity(),
        }
    }

    fn card_capacity_blocks(&self) -> u64 {
        match self {
            Csd::V1(csd) => csd.card_capacity_blocks(),
            Csd::V2(csd) => csd.card_capacity_blocks(),
        }
    }
}
