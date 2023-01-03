/// R1 response bitset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct R1Response(pub u8);

impl R1Response {
    /// In ready state.
    pub const READY_STATE: R1Response = R1Response(0x00);
    /// In idle state.
    pub const IN_IDLE_STATE: R1Response = R1Response(0x01);
    /// In idle state and illegal command.
    pub const IN_IDLE_AND_ILLEGAL: R1Response = R1Response(0x01 | 0x04);
    /// Invalid mask.
    const INVALID_MASK: u8 = 0x80;

    /// Check if R1 response is valid.
    pub fn is_valid(&self) -> bool {
        (self.0 & Self::INVALID_MASK) == 0x00
    }
}
