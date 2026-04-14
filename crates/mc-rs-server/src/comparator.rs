//! Redstone comparator — subtract / compare mode, container reading.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparatorMode {
    Compare,  // Output = max(rear) if side < rear else 0
    Subtract, // Output = max(rear - max_side, 0)
}

#[derive(Debug, Clone)]
pub struct Comparator {
    pub facing: u8,
    pub mode: ComparatorMode,
    pub rear_input: u8,
    pub left_input: u8,
    pub right_input: u8,
    pub output: u8,
}

impl Comparator {
    pub fn new(facing: u8) -> Self {
        Self {
            facing,
            mode: ComparatorMode::Compare,
            rear_input: 0,
            left_input: 0,
            right_input: 0,
            output: 0,
        }
    }

    pub fn recompute(&mut self) {
        let max_side = self.left_input.max(self.right_input);
        self.output = match self.mode {
            ComparatorMode::Compare => {
                if max_side <= self.rear_input {
                    self.rear_input
                } else {
                    0
                }
            }
            ComparatorMode::Subtract => self.rear_input.saturating_sub(max_side),
        };
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            ComparatorMode::Compare => ComparatorMode::Subtract,
            ComparatorMode::Subtract => ComparatorMode::Compare,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_passes_if_side_weak() {
        let mut c = Comparator::new(0);
        c.rear_input = 10;
        c.left_input = 5;
        c.recompute();
        assert_eq!(c.output, 10);
    }

    #[test]
    fn subtract_subtracts() {
        let mut c = Comparator::new(0);
        c.mode = ComparatorMode::Subtract;
        c.rear_input = 10;
        c.left_input = 3;
        c.recompute();
        assert_eq!(c.output, 7);
    }
}
