use bitfield::bitfield;

bitfield! {
    /// R1 response bitset.
    pub struct R1Response(u8);
    pub in_idle_state, _: 0;
    pub erase_reset, _: 1;
    pub illigal_command, _: 2;
    pub command_crc_error, _: 3;
    pub erase_sequence_error, _: 4;
    pub address_error, _: 5;
    pub parameter_error, _: 6;
}

/// R3 OCR payload.
pub type R3OcrPayload = [u8; 4];
