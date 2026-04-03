/// Merge-sort tree — stores a sorted copy of each segment at every node.
///
/// This makes it possible to answer "how many elements in [l, r] are less
/// than X?" in O(log² n) time, which in turn enables:
///
/// - k-th smallest element in an arbitrary subarray  O(log² n · log(max_val))
/// - count of elements less than / less-or-equal to X in [l, r]  O(log² n)
///
/// Space: O(n log n) — each element appears in O(log n) nodes.
///
/// Build: O(n log n) via the standard merge step.
///
/// # Example
/// ```
/// # use rust_trees::MergeSortTree;
/// let data = vec![3i32, 1, 4, 1, 5, 9, 2, 6];
/// let mst = MergeSortTree::new(&data);
/// assert_eq!(mst.count_less_than(0, 7, 5), 5); // {1,1,2,3,4}
/// assert_eq!(mst.kth_smallest(0, 7, 3), 2);    // sorted: [1,1,2,3,4,5,6,9] → 3rd = 2
/// ```
pub struct MergeSortTree {
    n:    usize,
    tree: Vec<Vec<i32>>, // 4n nodes; tree[v] = sorted elements of segment
}

impl MergeSortTree {
    /// Build the merge-sort tree from `data` in O(n log n).
    pub fn new(data: &[i32]) -> Self {
        let n = data.len();
        let mut tree = vec![Vec::new(); 4 * n];
        if n > 0 {
            Self::build(&mut tree, data, 1, 0, n - 1);
        }
        MergeSortTree { n, tree }
    }

    fn build(tree: &mut Vec<Vec<i32>>, data: &[i32], v: usize, l: usize, r: usize) {
        if l == r {
            tree[v] = vec![data[l]];
            return;
        }
        let m = (l + r) / 2;
        Self::build(tree, data, 2 * v, l, m);
        Self::build(tree, data, 2 * v + 1, m + 1, r);
        tree[v] = merge_sorted(&tree[2 * v], &tree[2 * v + 1]);
    }

    // --- internal query helpers --------------------------------------------

    /// Count elements < `val` in the canonical decomposition of [ql, qr].
    fn count_lt(&self, v: usize, l: usize, r: usize, ql: usize, qr: usize, val: i32) -> usize {
        if qr < l || r < ql {
            return 0;
        }
        if ql <= l && r <= qr {
            // Binary search: number of elements strictly less than `val`.
            return self.tree[v].partition_point(|&x| x < val);
        }
        let m = (l + r) / 2;
        self.count_lt(2 * v, l, m, ql, qr, val)
            + self.count_lt(2 * v + 1, m + 1, r, ql, qr, val)
    }

    /// Count elements ≤ `val` in [ql, qr].
    fn count_le(&self, v: usize, l: usize, r: usize, ql: usize, qr: usize, val: i32) -> usize {
        if qr < l || r < ql {
            return 0;
        }
        if ql <= l && r <= qr {
            return self.tree[v].partition_point(|&x| x <= val);
        }
        let m = (l + r) / 2;
        self.count_le(2 * v, l, m, ql, qr, val)
            + self.count_le(2 * v + 1, m + 1, r, ql, qr, val)
    }

    // --- public API --------------------------------------------------------

    /// Count elements strictly less than `val` in [l, r] (inclusive, 0-indexed).
    /// O(log² n).
    pub fn count_less_than(&self, l: usize, r: usize, val: i32) -> usize {
        assert!(l <= r && r < self.n, "range out of bounds");
        self.count_lt(1, 0, self.n - 1, l, r, val)
    }

    /// Count elements less than or equal to `val` in [l, r] (inclusive).
    /// O(log² n).
    pub fn count_less_or_equal(&self, l: usize, r: usize, val: i32) -> usize {
        assert!(l <= r && r < self.n, "range out of bounds");
        self.count_le(1, 0, self.n - 1, l, r, val)
    }

    /// Return the k-th smallest element (1-indexed) in [l, r].
    ///
    /// Strategy: binary search over the sorted values stored at the root
    /// (`tree[1]` = all elements sorted).  For each candidate value `v`,
    /// count how many elements in [l, r] are ≤ v.  The answer is the
    /// leftmost v with that count ≥ k.
    ///
    /// Complexity: O(log n · log² n) = O(log³ n).
    pub fn kth_smallest(&self, l: usize, r: usize, k: usize) -> i32 {
        assert!(l <= r && r < self.n, "range out of bounds");
        assert!(k >= 1 && k <= r - l + 1, "k out of range");

        // tree[1] contains every element sorted — use it as the value universe.
        let all_vals = &self.tree[1];
        let mut lo = 0usize;
        let mut hi = all_vals.len() - 1;
        while lo < hi {
            let mid = (lo + hi) / 2;
            // How many elements in [l, r] are ≤ all_vals[mid]?
            let cnt = self.count_le(1, 0, self.n - 1, l, r, all_vals[mid]);
            if cnt >= k {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        all_vals[lo]
    }

    pub fn len(&self) -> usize {
        self.n
    }
}

fn merge_sorted(a: &[i32], b: &[i32]) -> Vec<i32> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] <= b[j] {
            out.push(a[i]);
            i += 1;
        } else {
            out.push(b[j]);
            j += 1;
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
    out
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_less_than() {
        let data = vec![3i32, 1, 4, 1, 5, 9, 2, 6];
        let mst = MergeSortTree::new(&data);
        // Sorted: [1,1,2,3,4,5,6,9] — 5 elements < 5
        assert_eq!(mst.count_less_than(0, 7, 5), 5);
        // Subrange [2,5] = [4,1,5,9] — 2 elements < 5
        assert_eq!(mst.count_less_than(2, 5, 5), 2);
        assert_eq!(mst.count_less_than(0, 7, 1), 0);
        assert_eq!(mst.count_less_than(0, 7, 10), 8);
    }

    #[test]
    fn test_kth_smallest() {
        let data = vec![3i32, 1, 4, 1, 5, 9, 2, 6];
        let mst = MergeSortTree::new(&data);
        // Full range sorted: [1,1,2,3,4,5,6,9]
        assert_eq!(mst.kth_smallest(0, 7, 1), 1);
        assert_eq!(mst.kth_smallest(0, 7, 2), 1);
        assert_eq!(mst.kth_smallest(0, 7, 3), 2);
        assert_eq!(mst.kth_smallest(0, 7, 8), 9);
        // Subrange [0, 3] = {3,1,4,1} sorted: [1,1,3,4]
        assert_eq!(mst.kth_smallest(0, 3, 3), 3);
    }

    #[test]
    fn test_with_duplicates() {
        let data = vec![5i32, 5, 5, 5, 5];
        let mst = MergeSortTree::new(&data);
        assert_eq!(mst.count_less_than(0, 4, 5), 0);
        assert_eq!(mst.count_less_or_equal(0, 4, 5), 5);
        assert_eq!(mst.kth_smallest(0, 4, 3), 5);
    }

    #[test]
    fn test_single_element() {
        let data = vec![42i32];
        let mst = MergeSortTree::new(&data);
        assert_eq!(mst.kth_smallest(0, 0, 1), 42);
        assert_eq!(mst.count_less_than(0, 0, 42), 0);
        assert_eq!(mst.count_less_or_equal(0, 0, 42), 1);
    }
}
