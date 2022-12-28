pub mod commands {
    /// CMD base value.
    pub const CMD_BASE: u8 = 0x40;
    /// ACMD flag.
    pub const ACMD_FLAG: u8 = 0x80;
    /// GO_IDLE_STATE - init card in spi mode if CS low.
    pub const CMD0: u8 = CMD_BASE;
    /// SEND_IF_COND - verify SD Memory Card interface operating condition.
    pub const CMD8: u8 = CMD_BASE + 8;
    /// SEND_CSD - read the Card Specific Data (CSD register).
    pub const CMD9: u8 = CMD_BASE + 9;
    /// STOP_TRANSMISSION - end multiple block read sequence.
    pub const CMD12: u8 = CMD_BASE + 12;
    /// SEND_STATUS - read the card status register.
    pub const CMD13: u8 = CMD_BASE + 13;
    /// READ_SINGLE_BLOCK - read a single data block from the card.
    pub const CMD17: u8 = CMD_BASE + 17;
    /// READ_MULTIPLE_BLOCK - read a multiple data blocks from the card.
    pub const CMD18: u8 = CMD_BASE + 18;
    /// WRITE_BLOCK - write a single data block to the card.
    pub const CMD24: u8 = CMD_BASE + 24;
    /// WRITE_MULTIPLE_BLOCK - write blocks of data until a STOP_TRANSMISSION.
    pub const CMD25: u8 = CMD_BASE + 25;
    /// APP_CMD - escape for application specific command.
    pub const CMD55: u8 = CMD_BASE + 55;
    /// READ_OCR - read the OCR register of a card.
    pub const CMD58: u8 = CMD_BASE + 58;
    /// CRC_ON_OFF - enable or disable CRC checking.
    pub const CMD59: u8 = CMD_BASE + 59;
    /// SD_SEND_OP_COMD - Sends host capacity support information and activates
    /// the card's initialization process.
    pub const ACMD41: u8 = CMD_BASE + ACMD_FLAG + 41;
}

pub mod tokens {
    /// Start data token for read or write single block.
    pub const DATA_START_BLOCK: u8 = 0xFE;
    /// Stop token for write multiple blocks.
    pub const STOP_TRAN: u8 = 0xFD;
    /// Start data token for write multiple blocks.
    pub const WRITE_MULTIPLE: u8 = 0xFC;
    /// Mask for data response tokens after a write block operation.
    pub const DATA_RES_MASK: u8 = 0x1F;
    /// Write data accepted token.
    pub const DATA_RES_ACCEPTED: u8 = 0x05;
}
