/// Generic segment tree — iterative, padded to the next power-of-2.
///
/// Any associative binary operation with an identity element works:
/// sum, min, max, GCD, XOR, product, etc.
///
/// All indices are 0-based and inclusive.
///
/// # Example
/// ```
/// # use rust_trees::SegmentTree;
/// let data = vec![2i64, 7, 1, 8, 2, 8];
/// let mut st = SegmentTree::new(&data, 0, |a, b| a + b); // range-sum
/// assert_eq!(st.query(1, 4), 18);
/// st.update(2, 100);
/// assert_eq!(st.query(1, 4), 117);
/// ```
pub struct SegmentTree<T, F> {
    /// Padded size (next power of 2 >= original length)
    n: usize,
    data_len: usize,
    tree: Vec<T>,
    identity: T,
    combine: F,
}

impl<T, F> SegmentTree<T, F>
where
    T: Clone,
    F: Fn(&T, &T) -> T,
{
    pub fn new(data: &[T], identity: T, combine: F) -> Self {
        let data_len = data.len();
        let mut n = 1;
        while n < data_len {
            n <<= 1;
        }
        // Positions [n .. n+data_len) are leaves; [n+data_len .. 2n) are identity padding.
        let mut tree = vec![identity.clone(); 2 * n];
        for (i, v) in data.iter().enumerate() {
            tree[n + i] = v.clone();
        }
        for i in (1..n).rev() {
            let l = tree[2 * i].clone();
            let r = tree[2 * i + 1].clone();
            tree[i] = combine(&l, &r);
        }
        SegmentTree { n, data_len, tree, identity, combine }
    }

    /// Point update: set `data[pos] = val` in O(log n).
    pub fn update(&mut self, pos: usize, val: T) {
        assert!(pos < self.data_len, "index out of bounds");
        let mut i = self.n + pos;
        self.tree[i] = val;
        i >>= 1;
        while i >= 1 {
            let l = self.tree[2 * i].clone();
            let r = self.tree[2 * i + 1].clone();
            self.tree[i] = (self.combine)(&l, &r);
            i >>= 1;
        }
    }

    /// Range query on [l, r] (inclusive) in O(log n).
    ///
    /// Accumulates from both ends toward the middle so that the combine
    /// order matches [l .. mid] on the left and [mid .. r] on the right.
    pub fn query(&self, l: usize, r: usize) -> T {
        assert!(l <= r && r < self.data_len, "range out of bounds");
        let mut l = l + self.n;
        let mut r = r + self.n + 1; // exclusive upper bound
        let mut res_l = self.identity.clone();
        let mut res_r = self.identity.clone();
        while l < r {
            if l & 1 == 1 {
                // l is a right child — include it on the left accumulator
                res_l = (self.combine)(&res_l, &self.tree[l]);
                l += 1;
            }
            if r & 1 == 1 {
                // r-1 is a right child — include it on the right accumulator
                r -= 1;
                res_r = (self.combine)(&self.tree[r], &res_r);
            }
            l >>= 1;
            r >>= 1;
        }
        (self.combine)(&res_l, &res_r)
    }

    pub fn len(&self) -> usize {
        self.data_len
    }

    pub fn is_empty(&self) -> bool {
        self.data_len == 0
    }
}

// ---------------------------------------------------------------------------

/// Lazy propagation segment tree — concrete i64, range-add + range-sum.
///
/// Supports two operations in O(log n) each:
/// - `range_add(l, r, val)` — add `val` to every element in [l, r]
/// - `range_sum(l, r)`      — sum of elements in [l, r]
///
/// The lazy tag at each node stores a pending addition that has not yet
/// been pushed to children.
pub struct LazySegTree {
    n: usize,
    tree: Vec<i64>,
    lazy: Vec<i64>,
}

impl LazySegTree {
    pub fn new(data: &[i64]) -> Self {
        let n = data.len();
        let mut tree = vec![0i64; 4 * n];
        let lazy = vec![0i64; 4 * n];
        if n > 0 {
            Self::build(&mut tree, data, 1, 0, n - 1);
        }
        LazySegTree { n, tree, lazy }
    }

    fn build(tree: &mut [i64], data: &[i64], v: usize, l: usize, r: usize) {
        if l == r {
            tree[v] = data[l];
            return;
        }
        let m = (l + r) / 2;
        Self::build(tree, data, 2 * v, l, m);
        Self::build(tree, data, 2 * v + 1, m + 1, r);
        tree[v] = tree[2 * v] + tree[2 * v + 1];
    }

    /// Apply a pending addition to a node covering [l, r].
    fn apply(&mut self, v: usize, l: usize, r: usize, val: i64) {
        self.tree[v] += val * (r - l + 1) as i64;
        self.lazy[v] += val;
    }

    /// Push the lazy tag at `v` down to its two children.
    fn push(&mut self, v: usize, l: usize, r: usize) {
        if self.lazy[v] == 0 {
            return;
        }
        let m = (l + r) / 2;
        let pending = self.lazy[v];
        self.apply(2 * v, l, m, pending);
        self.apply(2 * v + 1, m + 1, r, pending);
        self.lazy[v] = 0;
    }

    fn do_range_add(&mut self, v: usize, l: usize, r: usize, ql: usize, qr: usize, val: i64) {
        if qr < l || r < ql {
            return;
        }
        if ql <= l && r <= qr {
            self.apply(v, l, r, val);
            return;
        }
        self.push(v, l, r);
        let m = (l + r) / 2;
        self.do_range_add(2 * v, l, m, ql, qr, val);
        self.do_range_add(2 * v + 1, m + 1, r, ql, qr, val);
        self.tree[v] = self.tree[2 * v] + self.tree[2 * v + 1];
    }

    fn do_range_sum(&mut self, v: usize, l: usize, r: usize, ql: usize, qr: usize) -> i64 {
        if qr < l || r < ql {
            return 0;
        }
        if ql <= l && r <= qr {
            return self.tree[v];
        }
        self.push(v, l, r);
        let m = (l + r) / 2;
        self.do_range_sum(2 * v, l, m, ql, qr)
            + self.do_range_sum(2 * v + 1, m + 1, r, ql, qr)
    }

    /// Add `val` to every element in [l, r] (inclusive, 0-indexed).
    pub fn range_add(&mut self, l: usize, r: usize, val: i64) {
        assert!(l <= r && r < self.n, "range out of bounds");
        self.do_range_add(1, 0, self.n - 1, l, r, val);
    }

    /// Return the sum of elements in [l, r] (inclusive, 0-indexed).
    pub fn range_sum(&mut self, l: usize, r: usize) -> i64 {
        assert!(l <= r && r < self.n, "range out of bounds");
        self.do_range_sum(1, 0, self.n - 1, l, r)
    }

    pub fn len(&self) -> usize {
        self.n
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_tree() {
        let data = vec![1i64, 3, 5, 7, 9, 11];
        let mut st = SegmentTree::new(&data, 0, |a, b| a + b);
        assert_eq!(st.query(0, 5), 36);
        assert_eq!(st.query(1, 3), 15);
        st.update(2, 10);
        assert_eq!(st.query(1, 3), 20);
    }

    #[test]
    fn test_min_tree() {
        let data = vec![4i32, 2, 7, 1, 9, 3];
        let st = SegmentTree::new(&data, i32::MAX, |a, b| *a.min(b));
        assert_eq!(st.query(0, 5), 1);
        assert_eq!(st.query(0, 2), 2);
        assert_eq!(st.query(3, 5), 1);
    }

    #[test]
    fn test_max_tree() {
        let data = vec![4i32, 2, 7, 1, 9, 3];
        let st = SegmentTree::new(&data, i32::MIN, |a, b| *a.max(b));
        assert_eq!(st.query(0, 5), 9);
        assert_eq!(st.query(0, 2), 7);
    }

    #[test]
    fn test_gcd_tree() {
        fn gcd(a: u64, b: u64) -> u64 {
            if b == 0 { a } else { gcd(b, a % b) }
        }
        let data = vec![12u64, 8, 6, 4];
        let st = SegmentTree::new(&data, 0u64, |a, b| gcd(*a, *b));
        assert_eq!(st.query(0, 3), 2);
        assert_eq!(st.query(0, 1), 4);
    }

    #[test]
    fn test_lazy_range_add_and_sum() {
        let data = vec![1i64, 2, 3, 4, 5];
        let mut st = LazySegTree::new(&data);
        assert_eq!(st.range_sum(0, 4), 15);
        st.range_add(1, 3, 10); // [1, 12, 13, 14, 5]
        assert_eq!(st.range_sum(0, 4), 45);
        assert_eq!(st.range_sum(1, 3), 39);
        st.range_add(0, 4, 1); // [2, 13, 14, 15, 6]
        assert_eq!(st.range_sum(0, 4), 50);
    }

    #[test]
    fn test_lazy_point_update_via_range() {
        let data = vec![0i64; 5];
        let mut st = LazySegTree::new(&data);
        st.range_add(2, 2, 7); // single-element range = point update
        assert_eq!(st.range_sum(0, 4), 7);
        assert_eq!(st.range_sum(2, 2), 7);
    }
}
