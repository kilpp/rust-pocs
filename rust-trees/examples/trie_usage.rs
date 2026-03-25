use rust_trees::Trie;

fn main() {
    println!("=== Trie (Prefix Tree) ===\n");

    let mut trie = Trie::new();
    let words = [
        "rust", "run", "runner", "running",
        "tree", "trie", "trim",
        "data", "database", "dataflow",
    ];

    for word in &words {
        trie.insert(word);
    }

    println!("Inserted {} words", trie.len());

    println!("\n--- Exact search ---");
    println!("contains 'rust'?    {}", trie.contains("rust"));
    println!("contains 'ru'?      {}", trie.contains("ru"));
    println!("contains 'running'? {}", trie.contains("running"));

    println!("\n--- Prefix check ---");
    println!("starts_with 'ru'?   {}", trie.starts_with("ru"));
    println!("starts_with 'xyz'?  {}", trie.starts_with("xyz"));

    println!("\n--- Autocomplete ---");
    println!("Words starting with 'run':  {:?}", trie.words_with_prefix("run"));
    println!("Words starting with 'data': {:?}", trie.words_with_prefix("data"));
    println!("Words starting with 'tr':   {:?}", trie.words_with_prefix("tr"));
}
