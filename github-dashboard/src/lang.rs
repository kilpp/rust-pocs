use std::collections::{HashMap, HashSet};

use crate::github::Pr;

/// Map a file extension (lowercase, no dot) to a language name.
fn ext_to_lang(ext: &str) -> Option<&'static str> {
    let lang = match ext {
        "rs" => "Rust",
        "py" | "pyi" => "Python",
        "ts" | "tsx" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "go" => "Go",
        "java" => "Java",
        "rb" => "Ruby",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "C++",
        "cs" => "C#",
        "php" => "PHP",
        "kt" | "kts" => "Kotlin",
        "swift" => "Swift",
        "scala" => "Scala",
        "sh" | "bash" | "zsh" => "Shell",
        "sql" => "SQL",
        "md" | "markdown" => "Markdown",
        "yml" | "yaml" => "YAML",
        "toml" => "TOML",
        "json" => "JSON",
        "html" | "htm" => "HTML",
        "css" | "scss" | "sass" => "CSS",
        "lua" => "Lua",
        "dart" => "Dart",
        "ex" | "exs" => "Elixir",
        "hs" => "Haskell",
        "ml" | "mli" => "OCaml",
        "vue" => "Vue",
        "proto" => "Protobuf",
        "tf" => "Terraform",
        _ => return None,
    };
    Some(lang)
}

/// Derive the distinct set of languages touched by a PR from its filenames.
/// Files with an unrecognised extension contribute "Other".
fn languages_for_files(files: &[String]) -> HashSet<String> {
    let mut langs = HashSet::new();
    for file in files {
        let ext = file
            .rsplit('.')
            .next()
            .filter(|e| !e.contains('/'))
            .map(str::to_ascii_lowercase);

        match ext.as_deref().and_then(ext_to_lang) {
            Some(lang) => {
                langs.insert(lang.to_string());
            }
            None => {
                langs.insert("Other".to_string());
            }
        }
    }
    langs
}

#[derive(Debug, Clone)]
pub struct LangStat {
    pub language: String,
    pub prs_open: usize,
    pub prs_closed: usize,
    pub prs: Vec<Pr>,
}

impl LangStat {
    pub fn total(&self) -> usize {
        self.prs_open + self.prs_closed
    }
}

/// Aggregate PRs (with their changed files) into per-language statistics.
/// A PR touching multiple languages counts once toward each of them.
/// Sorted by total PR count, descending.
pub fn aggregate(prs_with_files: Vec<(Pr, Vec<String>)>) -> Vec<LangStat> {
    let mut by_lang: HashMap<String, LangStat> = HashMap::new();

    for (pr, files) in prs_with_files {
        let langs = if files.is_empty() {
            // No file info (e.g. inaccessible PR) — still record it.
            HashSet::from(["Other".to_string()])
        } else {
            languages_for_files(&files)
        };

        for lang in langs {
            let entry = by_lang.entry(lang.clone()).or_insert_with(|| LangStat {
                language: lang,
                prs_open: 0,
                prs_closed: 0,
                prs: Vec::new(),
            });
            if pr.is_open() {
                entry.prs_open += 1;
            } else {
                entry.prs_closed += 1;
            }
            entry.prs.push(pr.clone());
        }
    }

    let mut stats: Vec<LangStat> = by_lang.into_values().collect();
    stats.sort_by(|a, b| {
        b.total()
            .cmp(&a.total())
            .then_with(|| a.language.cmp(&b.language))
    });
    stats
}
