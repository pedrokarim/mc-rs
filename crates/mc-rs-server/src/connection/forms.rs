use mc_rs_proto::io::ProtoReader;

use super::Connection;

impl Connection {
    /// Handler stub — on ne gère plus de formulaires côté serveur.
    /// Garde la signature pour que le dispatcher mod.rs compile.
    pub(super) fn handle_modal_form_response(&mut self, _reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        Vec::new()
    }
}
