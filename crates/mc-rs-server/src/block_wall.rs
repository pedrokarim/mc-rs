//! Wall connections — like fences but taller.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallConnection {
    None,
    Low,
    Tall,
}

#[derive(Debug, Clone)]
pub struct Wall {
    pub up: bool,
    pub north: WallConnection,
    pub south: WallConnection,
    pub east: WallConnection,
    pub west: WallConnection,
    pub waterlogged: bool,
}

impl Wall {
    pub fn new() -> Self {
        Self {
            up: true,
            north: WallConnection::None,
            south: WallConnection::None,
            east: WallConnection::None,
            west: WallConnection::None,
            waterlogged: false,
        }
    }

    /// Post appears when no direct connections on 2+ axes.
    pub fn has_post(&self) -> bool {
        let n = self.north != WallConnection::None;
        let s = self.south != WallConnection::None;
        let e = self.east != WallConnection::None;
        let w = self.west != WallConnection::None;
        // Has a post unless connections are exactly a straight line (N+S only, or E+W only).
        !((n && s && !e && !w) || (e && w && !n && !s))
    }
}

impl Default for Wall {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_post() {
        let w = Wall::new();
        assert!(w.has_post());
    }

    #[test]
    fn straight_ns_no_post() {
        let mut w = Wall::new();
        w.north = WallConnection::Low;
        w.south = WallConnection::Low;
        assert!(!w.has_post());
    }
}
