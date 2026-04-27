//! Sign block entity storage — texte par face d'un sign à une position.
//!
//! Format Bedrock (NBT compound) reçu via `BlockActorDataPacket` du client :
//! ```text
//! Compound {
//!   id: "Sign",
//!   x/y/z: int,
//!   FrontText: Compound {
//!     Text: String,
//!     SignTextColor: Int (-16777216 = black, etc),
//!     IgnoreLighting: Byte,
//!     PersistFormatting: Byte,
//!     HideGlowOutline: Byte,
//!   },
//!   BackText: Compound { Text, SignTextColor, ... },
//!   IsWaxed: Byte,
//! }
//! ```
//!
//! On stocke par position un `SignTextEntry` avec front + back. Le serveur
//! re-broadcast aux autres joueurs la même NBT (pour qu'ils voient la
//! modification).

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct SignFaceText {
    pub text: String,
    pub color: i32,
    pub ignore_lighting: bool,
    pub hide_glow_outline: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SignData {
    pub front: SignFaceText,
    pub back: SignFaceText,
    pub is_waxed: bool,
    /// NBT brut tel que reçu du client (utile pour broadcast verbatim).
    pub raw_nbt: Vec<u8>,
}

#[derive(Default)]
pub struct SignManager {
    signs: HashMap<(i32, i32, i32), SignData>,
}

impl SignManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, pos: (i32, i32, i32), data: SignData) {
        self.signs.insert(pos, data);
    }

    pub fn get(&self, pos: (i32, i32, i32)) -> Option<&SignData> {
        self.signs.get(&pos)
    }

    pub fn remove(&mut self, pos: (i32, i32, i32)) -> Option<SignData> {
        self.signs.remove(&pos)
    }

    pub fn count(&self) -> usize {
        self.signs.len()
    }
}

/// Décode un NBT compound de sign brut (réseau LE) → SignData.
/// Retourne None si le NBT n'est pas un sign valide.
pub fn parse_sign_nbt(raw: &[u8]) -> Option<SignData> {
    use bytes::Bytes;
    let mut buf = Bytes::copy_from_slice(raw);
    let root = mc_rs_nbt::read_nbt_network(&mut buf).ok()?;
    let compound = root.compound;

    let parse_face = |key: &str| -> SignFaceText {
        if let Some(mc_rs_nbt::NbtTag::Compound(c)) = compound.get(key) {
            let text = c
                .get("Text")
                .and_then(|t| match t {
                    mc_rs_nbt::NbtTag::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            let color = c
                .get("SignTextColor")
                .and_then(|t| match t {
                    mc_rs_nbt::NbtTag::Int(v) => Some(*v),
                    _ => None,
                })
                .unwrap_or(-16777216);
            let ignore_lighting = c
                .get("IgnoreLighting")
                .and_then(|t| match t {
                    mc_rs_nbt::NbtTag::Byte(v) => Some(*v != 0),
                    _ => None,
                })
                .unwrap_or(false);
            let hide_glow_outline = c
                .get("HideGlowOutline")
                .and_then(|t| match t {
                    mc_rs_nbt::NbtTag::Byte(v) => Some(*v != 0),
                    _ => None,
                })
                .unwrap_or(false);
            SignFaceText {
                text,
                color,
                ignore_lighting,
                hide_glow_outline,
            }
        } else {
            SignFaceText::default()
        }
    };

    let front = parse_face("FrontText");
    let back = parse_face("BackText");
    let is_waxed = compound
        .get("IsWaxed")
        .and_then(|t| match t {
            mc_rs_nbt::NbtTag::Byte(v) => Some(*v != 0),
            _ => None,
        })
        .unwrap_or(false);

    Some(SignData {
        front,
        back,
        is_waxed,
        raw_nbt: raw.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use mc_rs_nbt::{tag::NbtCompound, NbtRoot, NbtTag};

    #[test]
    fn parse_basic_sign() {
        let mut front = NbtCompound::new();
        front.insert("Text".into(), NbtTag::String("Hello".into()));
        front.insert("SignTextColor".into(), NbtTag::Int(-16777216));
        let mut compound = NbtCompound::new();
        compound.insert("FrontText".into(), NbtTag::Compound(front));
        compound.insert("IsWaxed".into(), NbtTag::Byte(0));
        let root = NbtRoot::new("", compound);

        let mut buf = BytesMut::new();
        mc_rs_nbt::write_nbt_network(&mut buf, &root);
        let bytes = buf.to_vec();

        let parsed = parse_sign_nbt(&bytes).expect("parsed");
        assert_eq!(parsed.front.text, "Hello");
        assert!(!parsed.is_waxed);
    }

    #[test]
    fn manager_roundtrip() {
        let mut mgr = SignManager::new();
        let data = SignData {
            front: SignFaceText {
                text: "Test".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        mgr.set((0, 64, 0), data);
        assert_eq!(mgr.count(), 1);
        assert_eq!(mgr.get((0, 64, 0)).unwrap().front.text, "Test");
    }
}
