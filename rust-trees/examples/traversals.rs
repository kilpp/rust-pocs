use rust_trees::BST;

fn main() {
    println!("=== Tree Traversal Comparison ===\n");

    //        50
    //       /  \
    //     30    70
    //    / \   / \
    //  20  40 60  80

    let tree: BST<i32> = vec![50, 30, 70, 20, 40, 60, 80].into_iter().collect();

    println!("Tree:");
    tree.pretty_print();

    println!("\n1. Inorder (Left → Root → Right)  → sorted output");
    println!("   {:?}", tree.inorder());

    println!("\n2. Preorder (Root → Left → Right)  → copy/serialize a tree");
    println!("   {:?}", tree.preorder());

    println!("\n3. Postorder (Left → Right → Root) → delete/free a tree");
    println!("   {:?}", tree.postorder());

    println!("\n4. Level-order (BFS)               → print by depth");
    for (i, level) in tree.level_order().iter().enumerate() {
        println!("   Depth {}: {:?}", i, level);
    }
}
