use std::time::Duration;

/// The 16-byte RakNet magic sequence used in all offline packets.
pub const RAKNET_MAGIC: [u8; 16] = [
    0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78,
];

/// RakNet protocol version for Bedrock Edition.
pub const RAKNET_PROTOCOL_VERSION: u8 = 11;

/// Maximum MTU the server will accept.
pub const MAX_MTU: u16 = 1492;

/// Minimum MTU the server will accept.
pub const MIN_MTU: u16 = 400;

/// Maximum overhead for a single frame within a FrameSet.
pub const MAX_FRAME_OVERHEAD: usize = 32;

/// Number of ordering channels supported.
pub const NUM_ORDER_CHANNELS: usize = 32;

/// If no data received from a session for this long, disconnect it.
pub const SESSION_TIMEOUT: Duration = Duration::from_secs(10);

/// How often to send ConnectedPing to keep the connection alive.
pub const PING_INTERVAL: Duration = Duration::from_secs(5);

/// How often the server processes ACKs, retransmissions, and timeouts.
pub const SERVER_TICK_INTERVAL: Duration = Duration::from_millis(50);

/// Time before retransmitting an unACKed FrameSet.
pub const RETRANSMIT_TIMEOUT: Duration = Duration::from_secs(1);

/// Maximum number of fragments allowed per split packet.
pub const MAX_SPLIT_COUNT: u32 = 512;

/// Time after which incomplete fragment assemblies are discarded.
pub const FRAGMENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum buffered out-of-order frames per ordering channel.
pub const MAX_ORDER_CHANNEL_BUFFER: usize = 256;

/// Size of the UDP receive buffer.
pub const RECV_BUF_SIZE: usize = 2048;
