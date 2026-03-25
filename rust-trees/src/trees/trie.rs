use std::collections::HashMap;

struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            is_end: false,
        }
    }
}

pub struct Trie {
    root: TrieNode,
    size: usize,
}

impl Trie {
    pub fn new() -> Self {
        Trie {
            root: TrieNode::new(),
            size: 0,
        }
    }

    pub fn insert(&mut self, word: &str) {
        let mut current = &mut self.root;
        for ch in word.chars() {
            current = current.children.entry(ch).or_insert_with(TrieNode::new);
        }
        if !current.is_end {
            current.is_end = true;
            self.size += 1;
        }
    }

    pub fn contains(&self, word: &str) -> bool {
        self.find_node(word).is_some_and(|n| n.is_end)
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        self.find_node(prefix).is_some()
    }

    fn find_node(&self, prefix: &str) -> Option<&TrieNode> {
        let mut current = &self.root;
        for ch in prefix.chars() {
            current = current.children.get(&ch)?;
        }
        Some(current)
    }

    pub fn words_with_prefix(&self, prefix: &str) -> Vec<String> {
        let mut results = Vec::new();
        if let Some(node) = self.find_node(prefix) {
            Self::collect_words(node, &mut prefix.to_string(), &mut results);
        }
        results
    }

    fn collect_words(node: &TrieNode, current: &mut String, results: &mut Vec<String>) {
        if node.is_end {
            results.push(current.clone());
        }
        let mut keys: Vec<char> = node.children.keys().copied().collect();
        keys.sort();
        for ch in keys {
            current.push(ch);
            Self::collect_words(&node.children[&ch], current, results);
            current.pop();
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_search() {
        let mut trie = Trie::new();
        trie.insert("hello");
        trie.insert("help");
        trie.insert("world");

        assert!(trie.contains("hello"));
        assert!(trie.contains("help"));
        assert!(!trie.contains("hel"));
        assert!(trie.starts_with("hel"));
        assert_eq!(trie.len(), 3);
    }

    #[test]
    fn test_prefix_search() {
        let mut trie = Trie::new();
        for word in &["car", "card", "care", "careful", "cars"] {
            trie.insert(word);
        }
        let mut results = trie.words_with_prefix("care");
        results.sort();
        assert_eq!(results, vec!["care", "careful"]);
    }
}
