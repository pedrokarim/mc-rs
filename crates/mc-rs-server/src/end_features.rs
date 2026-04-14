//! End features — end portal, end crystal, end gateway, end_rod lighting.

#[derive(Debug, Clone)]
pub struct EndPortalFrame {
    pub position: [i32; 3],
    pub has_eye: bool,
    pub facing: u8,
}

#[derive(Debug, Clone)]
pub struct EndGateway {
    pub position: [i32; 3],
    pub target_exit: Option<[i32; 3]>,
    pub age_ticks: u64,
}

impl EndGateway {
    pub fn is_ready_for_teleport(&self) -> bool {
        self.target_exit.is_some() && self.age_ticks > 20
    }
}

/// End island coordinate calculation (vanilla outer end city coords).
pub fn outer_end_city_coord(chunk_x: i32, chunk_z: i32) -> (i32, i32) {
    // Each outer island chunk is spaced out at ~430 blocks.
    (chunk_x * 430, chunk_z * 430)
}

/// End spawn platform (after dragon kill).
pub fn end_spawn_platform_position() -> [i32; 3] {
    [100, 50, 0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_ready_after_age() {
        let g = EndGateway {
            position: [0, 75, 0],
            target_exit: Some([5000, 64, 5000]),
            age_ticks: 100,
        };
        assert!(g.is_ready_for_teleport());
    }
}
