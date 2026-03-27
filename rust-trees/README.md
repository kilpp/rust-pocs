# rust-trees

Tree data structures implemented from scratch in Rust.

## Data Structures

### Binary Search Tree (BST)

A BST keeps elements sorted: for every node, all values in the left subtree are smaller and all values in the right subtree are larger. This gives O(log n) search, insert, min, and max on average — but degrades to O(n) if you insert sorted data (the tree becomes a linked list).

**Operations:** `insert`, `contains`, `min`, `max`, `len`, `height`, `pretty_print`
**Traversals:** `inorder` (sorted order), `preorder`, `postorder`, `level_order`

```rust
use rust_trees::BST;

let mut tree = BST::new();
tree.insert(5);
tree.insert(3);
tree.insert(7);

assert!(tree.contains(&5));
assert_eq!(tree.min(), Some(&3));
assert_eq!(tree.inorder(), vec![&3, &5, &7]);

// Or build from an iterator
let tree: BST<i32> = vec![8, 3, 10, 1, 6, 14].into_iter().collect();
tree.pretty_print();
```

### AVL Tree

A self-balancing BST. After every insertion, the tree rotates nodes to keep the height difference between left and right subtrees at most 1. This guarantees O(log n) operations even with sorted input.

**Operations:** `insert`, `contains`, `len`, `height`, `inorder`, `pretty_print`

```rust
use rust_trees::AVLTree;

// Inserting 1..=15 in order — a plain BST would have height 15, AVL keeps it at ~4
let tree: AVLTree<i32> = (1..=15).collect();
assert!(tree.height() <= 5);
assert_eq!(tree.len(), 15);

tree.pretty_print(); // shows height annotations per node
```

### Trie (Prefix Tree)

A tree where each edge represents a character. Used for fast prefix lookups on strings — `contains` and `starts_with` run in O(k) where k is the word length, regardless of how many words are stored.

**Operations:** `insert`, `contains`, `starts_with`, `words_with_prefix`, `len`

```rust
use rust_trees::Trie;

let mut trie = Trie::new();
trie.insert("rust");
trie.insert("run");
trie.insert("runner");

assert!(trie.contains("rust"));
assert!(!trie.contains("ru"));       // not a complete word
assert!(trie.starts_with("ru"));     // but is a valid prefix

let words = trie.words_with_prefix("run");
// => ["run", "runner"]
```

## Why Rust?

Trees are a good exercise for Rust's ownership model. Every node owns its children via `Box<Node<T>>`, and optional links are `Option<Box<Node<T>>>`. This means:

- **No null pointers** — `Option` makes the absence of a child explicit and compiler-checked.
- **No garbage collector** — memory is freed automatically when a node is dropped.
- **No data races** — the borrow checker ensures only one mutable reference exists at a time.
- **Generics with trait bounds** — `T: Ord + Display` lets the trees work with any comparable, printable type.

Recursive tree algorithms map cleanly to Rust's `match` on `Option`:

```rust
fn search(link: &Option<Box<Node<T>>>, value: &T) -> bool {
    match link {
        None => false,
        Some(node) => {
            if *value == node.value { true }
            else if *value < node.value { Self::search(&node.left, value) }
            else { Self::search(&node.right, value) }
        }
    }
}
```

The AVL tree is a particularly interesting case — Rust's ownership rules mean you can't just "swap pointers" for rotations. Instead, you `take()` ownership of children, restructure, and return the new subtree. This makes the rotation logic explicit and safe.

## Running

```sh
cargo run              # demo of all three trees
cargo run --example bst_usage
cargo run --example avl_usage
cargo run --example trie_usage
cargo run --example traversals
cargo test             # run all unit tests
```
