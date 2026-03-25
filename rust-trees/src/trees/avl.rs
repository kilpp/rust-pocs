use std::cmp;
use std::fmt;

type Link<T> = Option<Box<AVLNode<T>>>;

struct AVLNode<T> {
    value: T,
    left: Link<T>,
    right: Link<T>,
    height: i32,
}

impl<T> AVLNode<T> {
    fn new(value: T) -> Self {
        AVLNode {
            value,
            left: None,
            right: None,
            height: 1,
        }
    }
}

pub struct AVLTree<T> {
    root: Link<T>,
    size: usize,
}

fn height<T>(node: &Link<T>) -> i32 {
    node.as_ref().map_or(0, |n| n.height)
}

fn balance_factor<T>(node: &AVLNode<T>) -> i32 {
    height(&node.left) - height(&node.right)
}

fn update_height<T>(node: &mut AVLNode<T>) {
    node.height = 1 + cmp::max(height(&node.left), height(&node.right));
}

fn rotate_right<T>(mut root: Box<AVLNode<T>>) -> Box<AVLNode<T>> {
    let mut new_root = root.left.take().expect("rotate_right requires left child");
    root.left = new_root.right.take();
    update_height(&mut root);
    new_root.right = Some(root);
    update_height(&mut new_root);
    new_root
}

fn rotate_left<T>(mut root: Box<AVLNode<T>>) -> Box<AVLNode<T>> {
    let mut new_root = root.right.take().expect("rotate_left requires right child");
    root.right = new_root.left.take();
    update_height(&mut root);
    new_root.left = Some(root);
    update_height(&mut new_root);
    new_root
}

fn rebalance<T>(mut node: Box<AVLNode<T>>) -> Box<AVLNode<T>> {
    update_height(&mut node);
    let bf = balance_factor(&node);

    if bf > 1 {
        // Left-heavy
        if balance_factor(node.left.as_ref().unwrap()) < 0 {
            // Left-Right case
            node.left = Some(rotate_left(node.left.take().unwrap()));
        }
        return rotate_right(node);
    }

    if bf < -1 {
        // Right-heavy
        if balance_factor(node.right.as_ref().unwrap()) > 0 {
            // Right-Left case
            node.right = Some(rotate_right(node.right.take().unwrap()));
        }
        return rotate_left(node);
    }

    node
}

fn insert_node<T: Ord>(link: Link<T>, value: T) -> (Link<T>, bool) {
    match link {
        None => (Some(Box::new(AVLNode::new(value))), true),
        Some(mut node) => {
            let inserted;
            if value < node.value {
                let (new_left, ins) = insert_node(node.left.take(), value);
                node.left = new_left;
                inserted = ins;
            } else if value > node.value {
                let (new_right, ins) = insert_node(node.right.take(), value);
                node.right = new_right;
                inserted = ins;
            } else {
                return (Some(node), false); // duplicate
            }
            (Some(rebalance(node)), inserted)
        }
    }
}

impl<T: Ord + fmt::Display> AVLTree<T> {
    pub fn new() -> Self {
        AVLTree { root: None, size: 0 }
    }

    pub fn insert(&mut self, value: T) {
        let (new_root, inserted) = insert_node(self.root.take(), value);
        self.root = new_root;
        if inserted {
            self.size += 1;
        }
    }

    pub fn contains(&self, value: &T) -> bool {
        let mut current = &self.root;
        while let Some(node) = current {
            if *value == node.value {
                return true;
            } else if *value < node.value {
                current = &node.left;
            } else {
                current = &node.right;
            }
        }
        false
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn height(&self) -> i32 {
        height(&self.root)
    }

    pub fn inorder(&self) -> Vec<&T> {
        let mut result = Vec::new();
        Self::inorder_walk(&self.root, &mut result);
        result
    }

    fn inorder_walk<'a>(link: &'a Link<T>, out: &mut Vec<&'a T>) {
        if let Some(node) = link {
            Self::inorder_walk(&node.left, out);
            out.push(&node.value);
            Self::inorder_walk(&node.right, out);
        }
    }

    pub fn pretty_print(&self) {
        if let Some(ref root) = self.root {
            println!("{}(h={})", root.value, root.height);
            let has_left = root.left.is_some();
            let has_right = root.right.is_some();
            if has_left {
                Self::print_node(&root.left, "", has_right);
            }
            if has_right {
                Self::print_node(&root.right, "", false);
            }
        }
    }

    fn print_node(link: &Link<T>, prefix: &str, has_sibling: bool) {
        if let Some(node) = link {
            let connector = if has_sibling { "├── " } else { "└── " };
            println!(
                "{}{}{}(h={})",
                prefix, connector, node.value, node.height
            );

            let new_prefix = if has_sibling {
                format!("{}│   ", prefix)
            } else {
                format!("{}    ", prefix)
            };

            let has_left = node.left.is_some();
            let has_right = node.right.is_some();
            if has_left {
                Self::print_node(&node.left, &new_prefix, has_right);
            }
            if has_right {
                Self::print_node(&node.right, &new_prefix, false);
            }
        }
    }
}

impl<T: Ord + fmt::Display> Default for AVLTree<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord + fmt::Display> FromIterator<T> for AVLTree<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut tree = AVLTree::new();
        for item in iter {
            tree.insert(item);
        }
        tree
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avl_stays_balanced() {
        // Inserting sorted data into a BST would give height N,
        // but AVL keeps it O(log N)
        let tree: AVLTree<i32> = (1..=15).collect();
        assert_eq!(tree.len(), 15);
        assert!(tree.height() <= 5); // log2(15) ≈ 4
    }

    #[test]
    fn test_avl_inorder() {
        let tree: AVLTree<i32> = vec![10, 5, 15, 3, 7].into_iter().collect();
        let sorted: Vec<&i32> = tree.inorder();
        assert_eq!(sorted, vec![&3, &5, &7, &10, &15]);
    }

    #[test]
    fn test_avl_contains() {
        let tree: AVLTree<i32> = vec![20, 10, 30, 5, 15].into_iter().collect();
        assert!(tree.contains(&10));
        assert!(!tree.contains(&25));
    }
}
