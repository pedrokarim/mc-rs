//! Dragon egg — teleports when punched.

/// Teleport chance when right-clicked (always teleports when left-clicked).
pub const LEFT_CLICK_TELEPORT: bool = true;
/// Teleport range (max dist).
pub const TELEPORT_RANGE: i32 = 15;
/// Egg falls if gravity source destroyed below it.
pub const HAS_GRAVITY: bool = true;

#[cfg(test)]
mod tests {
    #[test]
    fn gravity_true() {
        assert!(super::HAS_GRAVITY);
    }
}
