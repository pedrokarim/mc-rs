use crate::io::reader::ProtoReadError;
use crate::io::{ProtoReader, ProtoWriter};

/// Server → client form request.
pub struct ModalFormRequest {
    pub form_id: u32,
    pub form_data: String,
}

impl ModalFormRequest {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(self.form_data.len() + 8);
        w.write_var_u32(self.form_id);
        w.write_string(&self.form_data);
        w.into_bytes()
    }
}

/// Client → server form response.
pub struct ModalFormResponse {
    pub form_id: u32,
    pub response_data: Option<String>,
    pub cancel_reason: Option<u8>,
}

impl ModalFormResponse {
    pub fn decode(reader: &mut ProtoReader) -> Result<Self, ProtoReadError> {
        let form_id = reader.read_var_u32()?;
        let response_data = if reader.read_bool()? {
            Some(reader.read_string()?)
        } else {
            None
        };

        let cancel_reason = if reader.remaining() > 0 && reader.read_bool()? {
            Some(reader.read_u8()?)
        } else {
            None
        };

        Ok(Self {
            form_id,
            response_data,
            cancel_reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_encodes_id_and_json() {
        let request = ModalFormRequest {
            form_id: 7,
            form_data: r#"{"type":"form"}"#.to_string(),
        };

        let encoded = request.encode();
        let mut reader = ProtoReader::new(&encoded);

        assert_eq!(reader.read_var_u32().unwrap(), 7);
        assert_eq!(reader.read_string().unwrap(), r#"{"type":"form"}"#);
    }

    #[test]
    fn response_decodes_selection() {
        let mut writer = ProtoWriter::new();
        writer.write_var_u32(12);
        writer.write_bool(true);
        writer.write_string("2");
        writer.write_bool(false);

        let bytes = writer.into_bytes();
        let response = ModalFormResponse::decode(&mut ProtoReader::new(&bytes)).unwrap();
        assert_eq!(response.form_id, 12);
        assert_eq!(response.response_data.as_deref(), Some("2"));
        assert_eq!(response.cancel_reason, None);
    }
}
