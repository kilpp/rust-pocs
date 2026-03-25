use rust_trees::AVLTree;

fn main() {
    println!("=== AVL Tree (Self-Balancing BST) ===\n");

    // Insert sorted data — a plain BST would degenerate to a linked list
    println!("Inserting 1..15 in order:");
    let tree: AVLTree<i32> = (1..=15).collect();

    tree.pretty_print();
    println!("\nSize: {}", tree.len());
    println!("Height: {} (log2(15) ≈ 4)", tree.height());
    println!("Inorder: {:?}", tree.inorder());

    // Compare with a smaller example showing rotations
    println!("\n--- Rotation demo ---");
    println!("Inserting 3, 2, 1 (triggers right rotation):");
    let small: AVLTree<i32> = vec![3, 2, 1].into_iter().collect();
    small.pretty_print();

    println!("\nInserting 1, 3, 2 (triggers left-right rotation):");
    let lr: AVLTree<i32> = vec![1, 3, 2].into_iter().collect();
    lr.pretty_print();
}
