//! Item stack merge / split logic.

/// Try to merge stack b into stack a. Returns remainder of b.
pub fn merge_stacks(a: &mut (u16, u16, u16), b: &mut (u16, u16, u16), max_stack: u16) {
    if !can_merge(a, b) {
        return;
    }
    let space = max_stack - a.1;
    let transfer = space.min(b.1);
    a.1 += transfer;
    b.1 -= transfer;
}

pub fn can_merge(a: &(u16, u16, u16), b: &(u16, u16, u16)) -> bool {
    a.0 == b.0 && a.2 == b.2
}

/// Split a stack in half (left click split).
pub fn split_half(stack: &mut (u16, u16, u16)) -> (u16, u16, u16) {
    let half = stack.1 / 2;
    let taken = stack.1 - half;
    stack.1 = half;
    (stack.0, taken, stack.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merging_same_id() {
        let mut a = (1u16, 10u16, 0u16);
        let mut b = (1u16, 20u16, 0u16);
        merge_stacks(&mut a, &mut b, 64);
        assert_eq!(a.1, 30);
        assert_eq!(b.1, 0);
    }

    #[test]
    fn max_stack_respected() {
        let mut a = (1u16, 60u16, 0u16);
        let mut b = (1u16, 20u16, 0u16);
        merge_stacks(&mut a, &mut b, 64);
        assert_eq!(a.1, 64);
        assert_eq!(b.1, 16);
    }
}
