//! Path finder — A* basique.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Node(pub i32, pub i32, pub i32);

#[derive(PartialEq, Eq)]
struct PriorityNode {
    cost: i32,
    node: Node,
}

impl Ord for PriorityNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost) // Min-heap
    }
}

impl PartialOrd for PriorityNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Manhattan heuristic.
pub fn manhattan_distance(a: Node, b: Node) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs() + (a.2 - b.2).abs()
}

/// Basic A* that assumes all blocks are walkable (for testing / not actual game logic).
pub fn find_path(
    start: Node,
    goal: Node,
    max_iterations: usize,
    walkable: impl Fn(Node) -> bool,
) -> Option<Vec<Node>> {
    let mut open = BinaryHeap::new();
    open.push(PriorityNode {
        cost: 0,
        node: start,
    });
    let mut came_from: HashMap<Node, Node> = HashMap::new();
    let mut g_score: HashMap<Node, i32> = HashMap::new();
    g_score.insert(start, 0);
    let mut iterations = 0;

    let neighbors = [
        (1, 0, 0),
        (-1, 0, 0),
        (0, 1, 0),
        (0, -1, 0),
        (0, 0, 1),
        (0, 0, -1),
    ];

    while let Some(PriorityNode { node, .. }) = open.pop() {
        iterations += 1;
        if iterations > max_iterations {
            return None;
        }
        if node == goal {
            let mut path = vec![node];
            let mut cur = node;
            while let Some(prev) = came_from.get(&cur) {
                path.push(*prev);
                cur = *prev;
            }
            path.reverse();
            return Some(path);
        }
        let cur_g = *g_score.get(&node).unwrap_or(&i32::MAX);
        for &(dx, dy, dz) in &neighbors {
            let next = Node(node.0 + dx, node.1 + dy, node.2 + dz);
            if !walkable(next) {
                continue;
            }
            let tentative = cur_g + 1;
            if tentative < *g_score.get(&next).unwrap_or(&i32::MAX) {
                came_from.insert(next, node);
                g_score.insert(next, tentative);
                let f = tentative + manhattan_distance(next, goal);
                open.push(PriorityNode {
                    cost: f,
                    node: next,
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_path_found() {
        let path = find_path(Node(0, 0, 0), Node(3, 0, 0), 100, |_| true);
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 4);
    }

    #[test]
    fn no_path_to_blocked() {
        let path = find_path(Node(0, 0, 0), Node(3, 0, 0), 100, |n| n == Node(0, 0, 0));
        assert!(path.is_none());
    }
}
