pub mod datagram;
pub mod offline;
pub mod online;
pub mod types;

/// RakNet packet IDs (from MessageIdentifiers.php)
pub mod id {
    pub const CONNECTED_PING: u8 = 0x00;
    pub const UNCONNECTED_PING: u8 = 0x01;
    pub const UNCONNECTED_PING_OPEN: u8 = 0x02;
    pub const CONNECTED_PONG: u8 = 0x03;
    pub const OPEN_CONNECTION_REQUEST_1: u8 = 0x05;
    pub const OPEN_CONNECTION_REPLY_1: u8 = 0x06;
    pub const OPEN_CONNECTION_REQUEST_2: u8 = 0x07;
    pub const OPEN_CONNECTION_REPLY_2: u8 = 0x08;
    pub const CONNECTION_REQUEST: u8 = 0x09;
    pub const CONNECTION_REQUEST_ACCEPTED: u8 = 0x10;
    pub const NEW_INCOMING_CONNECTION: u8 = 0x13;
    pub const DISCONNECTION_NOTIFICATION: u8 = 0x15;
    pub const INCOMPATIBLE_PROTOCOL_VERSION: u8 = 0x19;

    /// Datagram range (0x80 - 0x8F)
    pub const BITFLAG_VALID: u8 = 0x80;
    pub const BITFLAG_ACK: u8 = 0x40;
    pub const BITFLAG_NAK: u8 = 0x20;

    pub const ACK: u8 = BITFLAG_VALID | BITFLAG_ACK; // 0xC0
    pub const NACK: u8 = BITFLAG_VALID | BITFLAG_NAK; // 0xA0

    /// First user-defined packet ID
    pub const USER_PACKET_ENUM: u8 = 0x86;

    #[inline]
    pub fn is_datagram(id: u8) -> bool {
        id & BITFLAG_VALID != 0 && id & BITFLAG_ACK == 0 && id & BITFLAG_NAK == 0
    }

    #[inline]
    pub fn is_ack(id: u8) -> bool {
        id & BITFLAG_VALID != 0 && id & BITFLAG_ACK != 0
    }

    #[inline]
    pub fn is_nack(id: u8) -> bool {
        id & BITFLAG_VALID != 0 && id & BITFLAG_NAK != 0
    }
}
