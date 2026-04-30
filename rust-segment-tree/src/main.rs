use rust_segment_tree::SegmentTree;

fn main() {
    let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let mut tree = SegmentTree::new(&data);

    // sum on range [1, 3)  ->  2 + 3 = 5
    println!("{}", tree.query(1, 3));

    // overwrite element at index 2
    tree.update(2, 1);

    // sum on range [1, 3)  ->  2 + 1 = 3
    println!("{}", tree.query(1, 3));
}
