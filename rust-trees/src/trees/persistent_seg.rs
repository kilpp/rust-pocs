/// Persistent segment tree — arena-based path copying.
///
/// Every `update` allocates only O(log n) new nodes by copying only the
/// nodes along the path from root to the changed leaf; all other nodes are
/// shared between versions.  This gives us:
///
/// - Space:  O(n + k·log n) total for n initial elements and k updates
/// - Update: O(log n) — returns a new version index
/// - Query:  O(log n) per (version, range) pair
///
/// # Example
/// ```
/// # use rust_trees::PersistentSegTree;
/// let data = vec![1i64, 2, 3, 4, 5];
/// let mut pst = PersistentSegTree::new(&data);
/// let v1 = pst.update(0, 2, 99); // version 1: data[2] = 99
/// assert_eq!(pst.query(0, 0, 4), 15); // original unchanged
/// assert_eq!(pst.query(v1, 0, 4), 111);
/// ```

#[derive(Clone)]
struct PNode {
    left:  u32,   // index in the arena; 0 = null / empty subtree
    right: u32,
    sum:   i64,
}

/// Arena-based persistent segment tree over a 0-indexed array of i64.
pub struct PersistentSegTree {
    nodes: Vec<PNode>,
    roots: Vec<u32>,  // roots[version] = root node index
    n:     usize,
}

impl PersistentSegTree {
    pub fn new(data: &[i64]) -> Self {
        let n = data.len();
        // Index 0 is the null / empty node (sum = 0, no children).
        let mut seg = PersistentSegTree {
            nodes: vec![PNode { left: 0, right: 0, sum: 0 }],
            roots: Vec::new(),
            n,
        };
        if n > 0 {
            let root = seg.build(data, 0, n - 1);
            seg.roots.push(root);
        }
        seg
    }

    // --- internal helpers --------------------------------------------------

    fn alloc(&mut self, node: PNode) -> u32 {
        let idx = self.nodes.len() as u32;
        self.nodes.push(node);
        idx
    }

    fn build(&mut self, data: &[i64], l: usize, r: usize) -> u32 {
        if l == r {
            return self.alloc(PNode { left: 0, right: 0, sum: data[l] });
        }
        let m = (l + r) / 2;
        let left  = self.build(data, l, m);
        let right = self.build(data, m + 1, r);
        let sum   = self.nodes[left as usize].sum + self.nodes[right as usize].sum;
        self.alloc(PNode { left, right, sum })
    }

    /// Copy the path from `prev` down to `pos`, creating fresh nodes.
    /// All unchanged subtrees are reused (structural sharing).
    fn copy_update(&mut self, prev: u32, l: usize, r: usize, pos: usize, val: i64) -> u32 {
        if l == r {
            return self.alloc(PNode { left: 0, right: 0, sum: val });
        }
        let m = (l + r) / 2;
        // Clone prev's child pointers — we will overwrite one side.
        let PNode { left, right, .. } = self.nodes[prev as usize].clone();
        let (new_left, new_right) = if pos <= m {
            (self.copy_update(left, l, m, pos, val), right)
        } else {
            (left, self.copy_update(right, m + 1, r, pos, val))
        };
        let sum = self.nodes[new_left as usize].sum + self.nodes[new_right as usize].sum;
        self.alloc(PNode { left: new_left, right: new_right, sum })
    }

    fn range_sum_node(&self, node: u32, l: usize, r: usize, ql: usize, qr: usize) -> i64 {
        if node == 0 || qr < l || r < ql {
            return 0;
        }
        if ql <= l && r <= qr {
            return self.nodes[node as usize].sum;
        }
        let m = (l + r) / 2;
        let PNode { left, right, .. } = self.nodes[node as usize];
        self.range_sum_node(left, l, m, ql, qr)
            + self.range_sum_node(right, m + 1, r, ql, qr)
    }

    // --- public API --------------------------------------------------------

    /// Point-update: creates a new version from `version` with `data[pos] = val`.
    /// Returns the index of the newly created version.
    pub fn update(&mut self, version: usize, pos: usize, val: i64) -> usize {
        assert!(version < self.roots.len(), "unknown version");
        assert!(pos < self.n, "index out of bounds");
        let prev_root = self.roots[version];
        let new_root  = self.copy_update(prev_root, 0, self.n - 1, pos, val);
        self.roots.push(new_root);
        self.roots.len() - 1
    }

    /// Range sum of `data[l..=r]` at the given version.
    pub fn query(&self, version: usize, l: usize, r: usize) -> i64 {
        assert!(version < self.roots.len(), "unknown version");
        assert!(l <= r && r < self.n, "range out of bounds");
        let root = self.roots[version];
        self.range_sum_node(root, 0, self.n - 1, l, r)
    }

    /// How many versions (including the initial one) exist.
    pub fn num_versions(&self) -> usize {
        self.roots.len()
    }

    /// Total nodes allocated in the arena (useful for understanding sharing).
    pub fn node_count(&self) -> usize {
        self.nodes.len() - 1 // subtract the null sentinel
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_version() {
        let data = vec![1i64, 2, 3, 4, 5];
        let pst = PersistentSegTree::new(&data);
        assert_eq!(pst.num_versions(), 1);
        assert_eq!(pst.query(0, 0, 4), 15);
        assert_eq!(pst.query(0, 1, 3), 9);
    }

    #[test]
    fn test_versions_are_independent() {
        let data = vec![1i64, 2, 3, 4, 5];
        let mut pst = PersistentSegTree::new(&data);
        let v1 = pst.update(0, 2, 100); // [1,2,100,4,5]
        let v2 = pst.update(v1, 0, 50); // [50,2,100,4,5]
        let v3 = pst.update(0, 4, 999); // [1,2,3,4,999]

        assert_eq!(pst.query(0, 0, 4), 15);           // original
        assert_eq!(pst.query(v1, 0, 4), 112);         // +97 from 3→100
        assert_eq!(pst.query(v2, 0, 4), 161);         // +49 from 1→50
        assert_eq!(pst.query(v3, 0, 4), 1009);        // +994 from 5→999
        assert_eq!(pst.num_versions(), 4);
    }

    #[test]
    fn test_node_sharing() {
        // With n=8 and k updates we should use far fewer than k*n nodes.
        let data: Vec<i64> = (1..=8).collect();
        let mut pst = PersistentSegTree::new(&data);
        let initial_nodes = pst.node_count();
        for i in 0..8 {
            pst.update(0, i, 0);
        }
        // Each update adds only O(log n) = ~4 nodes, not O(n) = 15.
        let total_nodes = pst.node_count();
        assert!(total_nodes < initial_nodes + 8 * 5); // generous upper bound
    }
}
