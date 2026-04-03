use rust_trees::{LazySegTree, MergeSortTree, PersistentSegTree, SegmentTree};

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn main() {
    println!("╔══════════════════════════════════════════╗");
    println!("║        Segment Tree Variants             ║");
    println!("╚══════════════════════════════════════════╝\n");

    // -----------------------------------------------------------------------
    // 1. Generic SegmentTree — same structure, different operations
    // -----------------------------------------------------------------------
    println!("── 1. Generic SegmentTree ──");

    let scores = vec![4i64, 8, 15, 16, 23, 42];
    println!("Data: {:?}", scores);

    let mut sum_tree = SegmentTree::new(&scores, 0i64, |a, b| a + b);
    println!("Sum   [0..5] = {}", sum_tree.query(0, 5)); // 108
    println!("Sum   [1..4] = {}", sum_tree.query(1, 4)); // 62
    sum_tree.update(2, 100); // 15 → 100
    println!("After scores[2] = 100:");
    println!("Sum   [0..5] = {}", sum_tree.query(0, 5)); // 193

    let min_tree = SegmentTree::new(&scores, i64::MAX, |a, b| *a.min(b));
    println!("Min   [0..5] = {}", min_tree.query(0, 5)); // 4
    println!("Min   [2..5] = {}", min_tree.query(2, 5)); // 15

    let max_tree = SegmentTree::new(&scores, i64::MIN, |a, b| *a.max(b));
    println!("Max   [0..5] = {}", max_tree.query(0, 5)); // 42
    println!("Max   [0..2] = {}", max_tree.query(0, 2)); // 15

    let nums: Vec<u64> = vec![12, 8, 6, 4, 18];
    let gcd_tree = SegmentTree::new(&nums, 0u64, |a, b| gcd(*a, *b));
    println!("GCD   [0..4] = {}", gcd_tree.query(0, 4)); // 2
    println!("GCD   [0..2] = {}", gcd_tree.query(0, 2)); // 2  (gcd(12,8,6))
    println!("GCD   [2..4] = {}", gcd_tree.query(2, 4)); // 2  (gcd(6,4,18))

    // -----------------------------------------------------------------------
    // 2. LazySegTree — range additions with O(log n) cost per op
    // -----------------------------------------------------------------------
    println!("\n── 2. LazySegTree (range-add + range-sum) ──");

    // Scenario: a company gives department-wide salary raises.
    // salaries[i] = base salary of employee i (in $k)
    let salaries = vec![50i64, 60, 70, 80, 90, 100]; // employees 0..5
    println!("Initial salaries: {:?}", salaries);
    let mut lazy = LazySegTree::new(&salaries);

    println!("Total payroll [0..5]: ${}", lazy.range_sum(0, 5)); // 450

    // Give employees 1–3 a $5k raise
    lazy.range_add(1, 3, 5);
    println!("After $5k raise for employees 1–3:");
    println!("  Dept 1-3 payroll: ${}", lazy.range_sum(1, 3)); // 225
    println!("  Total payroll:    ${}", lazy.range_sum(0, 5)); // 465

    // Give everyone a $2k raise (company-wide)
    lazy.range_add(0, 5, 2);
    println!("After $2k company-wide raise:");
    println!("  Total payroll:    ${}", lazy.range_sum(0, 5)); // 477

    // -----------------------------------------------------------------------
    // 3. PersistentSegTree — branching timeline of array versions
    // -----------------------------------------------------------------------
    println!("\n── 3. PersistentSegTree (versioned array) ──");

    // Scenario: simulate git-like snapshots of a configuration array.
    let config = vec![1i64, 2, 3, 4, 5];
    println!("Initial config (v0): {:?}", config);
    let mut pst = PersistentSegTree::new(&config);

    // v1: patch index 2 (3 → 99)
    let v1 = pst.update(0, 2, 99);
    // v2: based on v1, also patch index 0 (1 → 50)
    let v2 = pst.update(v1, 0, 50);
    // v3: independent hotfix on v0 — patch index 4 (5 → 999)
    let v3 = pst.update(0, 4, 999);

    println!("v0 sum [0..4] = {}", pst.query(0, 0, 4));  // 15
    println!("v1 sum [0..4] = {}", pst.query(v1, 0, 4)); // 111  (+96)
    println!("v2 sum [0..4] = {}", pst.query(v2, 0, 4)); // 160  (+49)
    println!("v3 sum [0..4] = {}", pst.query(v3, 0, 4)); // 1009 (+994)
    println!("Versions: {}  |  Arena nodes: {}", pst.num_versions(), pst.node_count());
    // With n=5 the full tree has 9 nodes; each update adds ~log₂(5)≈3 nodes.
    // Total ≈ 9 + 3×3 = 18, far less than 4 × 9 = 36.

    // -----------------------------------------------------------------------
    // 4. MergeSortTree — k-th smallest in arbitrary subarray
    // -----------------------------------------------------------------------
    println!("\n── 4. MergeSortTree (k-th smallest in range) ──");

    // Scenario: leaderboard scores; answer "what rank-k score sits in
    // positions [l, r] of the leaderboard?"
    let board = vec![34i32, 7, 23, 32, 5, 62, 78, 14, 43, 21];
    println!("Board: {:?}", board);
    let mst = MergeSortTree::new(&board);

    // Full leaderboard sorted: [5,7,14,21,23,32,34,43,62,78]
    println!("1st smallest overall:       {}", mst.kth_smallest(0, 9, 1));  // 5
    println!("5th smallest overall:       {}", mst.kth_smallest(0, 9, 5));  // 23
    println!("10th smallest overall:      {}", mst.kth_smallest(0, 9, 10)); // 78

    // Subrange [2, 6] = {23, 32, 5, 62, 78} sorted: [5, 23, 32, 62, 78]
    println!("2nd smallest in [2..6]:     {}", mst.kth_smallest(2, 6, 2)); // 23
    println!("3rd smallest in [2..6]:     {}", mst.kth_smallest(2, 6, 3)); // 32

    // Count queries
    let lt50 = mst.count_less_than(0, 9, 50);
    let le50 = mst.count_less_or_equal(0, 9, 50);
    println!("Elements < 50 overall:      {}", lt50); // 8
    println!("Elements ≤ 50 overall:      {}", le50); // 8  (no element equals 50)
    println!("Elements < 50 in [3..7]:    {}", mst.count_less_than(3, 7, 50)); // 3
}
