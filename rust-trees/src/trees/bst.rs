use std::fmt;

type Link<T> = Option<Box<Node<T>>>;

struct Node<T> {
    value: T,
    left: Link<T>,
    right: Link<T>,
}

pub struct BST<T> {
    root: Link<T>,
    size: usize,
}

impl<T: Ord + fmt::Display> BST<T> {
    pub fn new() -> Self {
        BST { root: None, size: 0 }
    }

    pub fn insert(&mut self, value: T) {
        Self::insert_into(&mut self.root, value);
        self.size += 1;
    }

    fn insert_into(link: &mut Link<T>, value: T) {
        match link {
            None => {
                *link = Some(Box::new(Node {
                    value,
                    left: None,
                    right: None,
                }));
            }
            Some(node) => {
                if value < node.value {
                    Self::insert_into(&mut node.left, value);
                } else if value > node.value {
                    Self::insert_into(&mut node.right, value);
                }
            }
        }
    }

    pub fn contains(&self, value: &T) -> bool {
        Self::search(&self.root, value)
    }

    fn search(link: &Link<T>, value: &T) -> bool {
        match link {
            None => false,
            Some(node) => {
                if *value == node.value {
                    true
                } else if *value < node.value {
                    Self::search(&node.left, value)
                } else {
                    Self::search(&node.right, value)
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn min(&self) -> Option<&T> {
        Self::find_min(&self.root)
    }

    fn find_min(link: &Link<T>) -> Option<&T> {
        match link {
            None => None,
            Some(node) => {
                if node.left.is_none() {
                    Some(&node.value)
                } else {
                    Self::find_min(&node.left)
                }
            }
        }
    }

    pub fn max(&self) -> Option<&T> {
        Self::find_max(&self.root)
    }

    fn find_max(link: &Link<T>) -> Option<&T> {
        match link {
            None => None,
            Some(node) => {
                if node.right.is_none() {
                    Some(&node.value)
                } else {
                    Self::find_max(&node.right)
                }
            }
        }
    }

    // --- Traversals ---

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

    pub fn preorder(&self) -> Vec<&T> {
        let mut result = Vec::new();
        Self::preorder_walk(&self.root, &mut result);
        result
    }

    fn preorder_walk<'a>(link: &'a Link<T>, out: &mut Vec<&'a T>) {
        if let Some(node) = link {
            out.push(&node.value);
            Self::preorder_walk(&node.left, out);
            Self::preorder_walk(&node.right, out);
        }
    }

    pub fn postorder(&self) -> Vec<&T> {
        let mut result = Vec::new();
        Self::postorder_walk(&self.root, &mut result);
        result
    }

    fn postorder_walk<'a>(link: &'a Link<T>, out: &mut Vec<&'a T>) {
        if let Some(node) = link {
            Self::postorder_walk(&node.left, out);
            Self::postorder_walk(&node.right, out);
            out.push(&node.value);
        }
    }

    pub fn level_order(&self) -> Vec<Vec<&T>> {
        let mut levels: Vec<Vec<&T>> = Vec::new();
        if self.root.is_none() {
            return levels;
        }
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(self.root.as_ref().unwrap().as_ref());

        while !queue.is_empty() {
            let level_size = queue.len();
            let mut level = Vec::new();
            for _ in 0..level_size {
                let node = queue.pop_front().unwrap();
                level.push(&node.value);
                if let Some(ref left) = node.left {
                    queue.push_back(left.as_ref());
                }
                if let Some(ref right) = node.right {
                    queue.push_back(right.as_ref());
                }
            }
            levels.push(level);
        }
        levels
    }

    pub fn height(&self) -> usize {
        Self::node_height(&self.root)
    }

    fn node_height(link: &Link<T>) -> usize {
        match link {
            None => 0,
            Some(node) => {
                1 + std::cmp::max(
                    Self::node_height(&node.left),
                    Self::node_height(&node.right),
                )
            }
        }
    }

    pub fn pretty_print(&self) {
        if let Some(ref root) = self.root {
            println!("{}", root.value);
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
            println!("{}{}{}", prefix, connector, node.value);

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

impl<T: Ord + fmt::Display> Default for BST<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord + fmt::Display> FromIterator<T> for BST<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut tree = BST::new();
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
    fn test_insert_and_contains() {
        let mut tree = BST::new();
        tree.insert(5);
        tree.insert(3);
        tree.insert(7);
        assert!(tree.contains(&5));
        assert!(tree.contains(&3));
        assert!(!tree.contains(&4));
        assert_eq!(tree.len(), 3);
    }

    #[test]
    fn test_inorder() {
        let tree: BST<i32> = vec![5, 3, 7, 1, 4].into_iter().collect();
        let sorted: Vec<&i32> = tree.inorder();
        assert_eq!(sorted, vec![&1, &3, &4, &5, &7]);
    }

    #[test]
    fn test_min_max() {
        let tree: BST<i32> = vec![5, 3, 7, 1, 9].into_iter().collect();
        assert_eq!(tree.min(), Some(&1));
        assert_eq!(tree.max(), Some(&9));
    }
}
