//! Slab blocks — top, bottom, double.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabPosition {
    Bottom,
    Top,
    Double,
}

#[derive(Debug, Clone)]
pub struct Slab {
    pub material: u16,
    pub position: SlabPosition,
    pub waterlogged: bool,
}

impl Slab {
    pub fn new(material: u16) -> Self {
        Self {
            material,
            position: SlabPosition::Bottom,
            waterlogged: false,
        }
    }

    /// Stacking 2 slabs of same type = double slab (full block).
    pub fn try_stack_same_material(&self, other_position: SlabPosition) -> Option<Self> {
        if self.position == SlabPosition::Double {
            return None;
        }
        if (self.position == SlabPosition::Bottom && other_position == SlabPosition::Top)
            || (self.position == SlabPosition::Top && other_position == SlabPosition::Bottom)
        {
            Some(Self {
                material: self.material,
                position: SlabPosition::Double,
                waterlogged: self.waterlogged,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_plus_top_equals_double() {
        let s = Slab::new(1);
        let stacked = s.try_stack_same_material(SlabPosition::Top);
        assert!(stacked.is_some());
        assert_eq!(stacked.unwrap().position, SlabPosition::Double);
    }
}
