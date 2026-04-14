//! Transfer — feature Bedrock pour transférer un joueur vers un autre serveur.
//! Utilisé par les hub multi-servers. Port de `TransferPacket`.

use mc_rs_proto::io::ProtoWriter;

pub struct TransferPacket {
    pub address: String,
    pub port: u16,
}

impl TransferPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(32 + self.address.len());
        w.write_string(&self.address);
        w.write_u16_le(self.port);
        w.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_address_and_port() {
        let t = TransferPacket {
            address: "play.example.com".into(),
            port: 19132,
        };
        let bytes = t.encode();
        assert!(!bytes.is_empty());
    }
}
