/// RakNet protocol version used by MCPE
pub const RAKNET_PROTOCOL_VERSION: u8 = 11;

/// Magic bytes present in all offline RakNet messages
pub const MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

/// Packet ID for MCPE game packets wrapped in RakNet
pub const MCPE_PACKET_ID: u8 = 0xFE;

/// Minimum MTU size for a session
pub const MIN_MTU_SIZE: u16 = 400;

/// Maximum MTU size
pub const MAX_MTU_SIZE: u16 = 1492;

/// IP + UDP header overhead
pub const IP_UDP_HEADER_SIZE: u16 = 28;

/// Datagram header: flags (1) + seqNumber (3)
pub const DATAGRAM_HEADER_SIZE: usize = 4;

/// Extra RakNet overhead subtracted from MTU for payload calculation
/// IP(20) + UDP(8) + RakNet(8) = 36, plus datagram header(4) = 40
pub const DATAGRAM_MTU_OVERHEAD: usize = 40;

/// Number of system addresses written in ConnectionRequestAccepted (MCPE uses 20)
pub const SYSTEM_ADDRESS_COUNT: usize = 20;

/// Session timeout in seconds
pub const SESSION_TIMEOUT_SECS: f64 = 10.0;

/// Ping interval in seconds
pub const PING_INTERVAL_SECS: f64 = 5.0;

/// Reliable packet retransmit timeout in seconds
pub const RETRANSMIT_TIMEOUT_SECS: f64 = 2.0;

/// Receive sequence number window size
pub const RECV_WINDOW_SIZE: u32 = 2048;

/// Send reliable window size
pub const SEND_RELIABLE_WINDOW_SIZE: u32 = 512;

/// Max split parts per packet
pub const MAX_SPLIT_PART_COUNT: u32 = 128;

/// Max concurrent split packets being reassembled
pub const MAX_CONCURRENT_SPLITS: usize = 4;

/// Max order channels
pub const MAX_ORDER_CHANNELS: usize = 32;
