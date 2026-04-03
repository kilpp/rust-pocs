use rust_trees::{AVLTree, BST, Trie};

fn main() {
    println!("╔══════════════════════════════════╗");
    println!("║     Rust Tree Data Structures    ║");
    println!("╚══════════════════════════════════╝\n");

    // --- BST ---
    println!("── Binary Search Tree ──");
    let bst: BST<i32> = vec![8, 3, 10, 1, 6, 14].into_iter().collect();
    bst.pretty_print();
    println!("Sorted: {:?}\n", bst.inorder());

    // --- AVL ---
    println!("── AVL Tree (1..10 inserted in order) ──");
    let avl: AVLTree<i32> = (1..=10).collect();
    avl.pretty_print();
    println!("Height: {} (balanced)\n", avl.height());

    // --- Trie ---
    println!("── Trie ──");
    let mut trie = Trie::new();
    for w in &["rust", "run", "runner", "tree", "trie"] {
        trie.insert(w);
    }
    println!("Words with 'ru': {:?}", trie.words_with_prefix("ru"));
    println!("Words with 'tr': {:?}", trie.words_with_prefix("tr"));

    println!("\nRun individual examples for more detail:");
    println!("  cargo run --example bst_usage");
    println!("  cargo run --example avl_usage");
    println!("  cargo run --example trie_usage");
    println!("  cargo run --example traversals");
    println!("  cargo run --example segtree_usage");
}
