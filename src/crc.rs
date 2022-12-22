use crc_any::{CRCu16, CRCu8};

/// CRC-7 calculation.
pub fn crc7(data: &[u8]) -> u8 {
    let mut crc = CRCu8::crc7();

    crc.digest(data);

    crc.get_crc()
}

/// CRC-16 calculation.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = CRCu16::create_crc(0x1021, 16, 0x0000, 0x0000, false);

    crc.digest(data);

    crc.get_crc()
}
