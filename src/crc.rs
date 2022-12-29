/// CRC-7 calculation.
pub fn crc7(data: &[u8]) -> u8 {
    let mut crc = 0;
    for mut byte in data.iter().cloned() {
        for _bit in 0..8 {
            crc <<= 1;
            if ((byte & 0x80) ^ (crc & 0x80)) != 0 {
                crc ^= 0x09;
            }
            byte <<= 1;
        }
    }
    crc
}

/// CRC-16 calculation.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0;
    for &byte in data {
        crc = ((crc >> 8) & 0xFF) | (crc << 8);
        crc ^= u16::from(byte);
        crc ^= (crc & 0xFF) >> 4;
        crc ^= crc << 12;
        crc ^= (crc & 0xFF) << 5;
    }
    crc
}
