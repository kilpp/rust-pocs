use rust_trees::BST;

fn main() {
    println!("=== Binary Search Tree ===\n");

    // Build from iterator
    let tree: BST<i32> = vec![8, 3, 10, 1, 6, 14, 4, 7, 13].into_iter().collect();

    println!("Tree structure:");
    tree.pretty_print();

    println!("\nSize: {}", tree.len());
    println!("Height: {}", tree.height());
    println!("Min: {:?}", tree.min());
    println!("Max: {:?}", tree.max());

    println!("\nContains 6? {}", tree.contains(&6));
    println!("Contains 5? {}", tree.contains(&5));

    println!("\nInorder:   {:?}", tree.inorder());
    println!("Preorder:  {:?}", tree.preorder());
    println!("Postorder: {:?}", tree.postorder());

    println!("\nLevel-order:");
    for (i, level) in tree.level_order().iter().enumerate() {
        println!("  Level {}: {:?}", i, level);
    }
}
