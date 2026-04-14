//! RCON protocol (remote console).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RconPacketType {
    Login = 3,
    Command = 2,
    ResponseValue = 0,
}

#[derive(Debug, Clone)]
pub struct RconPacket {
    pub length: i32,
    pub request_id: i32,
    pub packet_type: RconPacketType,
    pub payload: String,
}

/// Max packet size (4110 bytes).
pub const MAX_PACKET_SIZE: usize = 4110;

impl RconPacket {
    pub fn new_login(request_id: i32, password: &str) -> Self {
        Self {
            length: 10 + password.len() as i32,
            request_id,
            packet_type: RconPacketType::Login,
            payload: password.to_string(),
        }
    }

    pub fn new_command(request_id: i32, cmd: &str) -> Self {
        Self {
            length: 10 + cmd.len() as i32,
            request_id,
            packet_type: RconPacketType::Command,
            payload: cmd.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_packet_has_correct_length() {
        let p = RconPacket::new_login(1, "password");
        assert_eq!(p.length, 18);
    }
}
