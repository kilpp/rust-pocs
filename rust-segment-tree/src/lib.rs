
pub struct SegmentTree {
    /// Number of leaves
    n: usize,
    tree: Vec<i32>,
}

impl SegmentTree {
    pub fn new(arr: &[i32]) -> Self {
        assert!(!arr.is_empty(), "arr must be non-empty");

        let n = arr.len();
        let mut tree = vec![0; 2 * n];

        // Step 1: copy input values into the leaf range [..., arr]
        tree[n..].copy_from_slice(arr);


        // Step 2: build the internal nodes by summing children
        //Example i = 5 -> tree[5] = tree[10] + tree[11] = 5 + 6 = 11
        for i in (1..n).rev() {
            tree[i] = tree[2 * i] + tree[2 * i + 1];
        }

        Self { n, tree }
    }

    pub fn size(&self) -> usize {
        self.n
    }

    //just to show in the tui
    pub fn nodes(&self) -> &[i32] {
        &self.tree
    }


    //just to show in the tui
    pub fn update_traced(&mut self, p: usize, value: i32) -> Vec<usize> {
        assert!(p < self.n, "index {p} out of bounds for length {}", self.n);

        let mut path = Vec::new();
        let mut i = p + self.n;
        self.tree[i] = value;
        path.push(i);
        i >>= 1;
        while i > 0 {
            self.tree[i] = self.tree[2 * i] + self.tree[2 * i + 1];
            path.push(i);
            i >>= 1;
        }
        path
    }

    //just to show in the tui
    pub fn query_traced(&self, l: usize, r: usize) -> (i32, Vec<usize>) {
        assert!(
            l <= r && r <= self.n,
            "range [{l}, {r}) out of bounds for length {}",
            self.n
        );

        let mut sum = 0;
        let mut visited = Vec::new();
        let mut l = l + self.n;
        let mut r = r + self.n;
        while l < r {
            if l & 1 == 1 {
                sum += self.tree[l];
                visited.push(l);
                l += 1;
            }
            if r & 1 == 1 {
                r -= 1;
                sum += self.tree[r];
                visited.push(r);
            }
            l >>= 1;
            r >>= 1;
        }
        (sum, visited)
    }

    /// Replaces the value at index p with value and refreshes all
    pub fn update(&mut self, p: usize, value: i32) {
        assert!(p < self.n, "index {p} out of bounds for length {}", self.n);

        
        let mut i = p + self.n;
        self.tree[i] = value;
        i >>= 1;
        while i > 0 {
            self.tree[i] = self.tree[2 * i] + self.tree[2 * i + 1];
            i >>= 1;
        }
    }

    pub fn query(&self, l: usize, r: usize) -> i32 {
        assert!(
            l <= r && r <= self.n,
            "range [{l}, {r}) out of bounds for length {}",
            self.n
        );

        let mut sum = 0;
        let mut l = l + self.n;
        let mut r = r + self.n;
        while l < r {
            if l & 1 == 1 {
                sum += self.tree[l];
                l += 1;
            }
            if r & 1 == 1 {
                r -= 1;
                sum += self.tree[r];
            }
            l >>= 1;
            r >>= 1;
        }
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::SegmentTree;

    #[test]
    fn gfg_example() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let mut st = SegmentTree::new(&a);

        assert_eq!(st.query(1, 3), 5); // 2 + 3
        st.update(2, 1);
        assert_eq!(st.query(1, 3), 3); // 2 + 1
    }

    #[test]
    fn full_range_sum() {
        let st = SegmentTree::new(&[1, 2, 3, 4, 5]);
        assert_eq!(st.query(0, 5), 15);
    }

    #[test]
    fn empty_range_is_zero() {
        let st = SegmentTree::new(&[4, 2, 7]);
        assert_eq!(st.query(1, 1), 0);
    }

    #[test]
    fn single_element_range() {
        let st = SegmentTree::new(&[4, 2, 7]);
        assert_eq!(st.query(1, 2), 2);
    }

    #[test]
    fn update_then_query() {
        let mut st = SegmentTree::new(&[1, 1, 1, 1, 1]);
        st.update(0, 10);
        st.update(4, 10);
        assert_eq!(st.query(0, 5), 23);
        assert_eq!(st.query(1, 4), 3);
    }

    #[test]
    #[should_panic]
    fn query_right_out_of_bounds() {
        let st = SegmentTree::new(&[1, 2, 3]);
        st.query(0, 4);
    }

    #[test]
    #[should_panic]
    fn query_inverted_range() {
        let st = SegmentTree::new(&[1, 2, 3]);
        st.query(2, 1);
    }

    #[test]
    #[should_panic]
    fn update_out_of_bounds() {
        let mut st = SegmentTree::new(&[1, 2, 3]);
        st.update(3, 0);
    }

    #[test]
    #[should_panic(expected = "arr must be non-empty")]
    fn empty_array_rejected() {
        let _ = SegmentTree::new(&[]);
    }
}
