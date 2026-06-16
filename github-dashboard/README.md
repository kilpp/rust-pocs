# github-dashboard

A terminal dashboard (Rust + [ratatui](https://ratatui.rs)) that reports GitHub
contribution activity for a configurable set of users over a configurable
timeline, broken down by programming language — and can ask the local `claude`
CLI what a given language's pull requests were about.

## Features

- **Configurable users & endpoint** — track any GitHub usernames; works against
  public GitHub or a GitHub Enterprise server (`base_url`).
- **Configurable timeline** — default last 7 days, overridable with `--days`.
- **Contributions** — GitHub-style contribution totals per user (the same data
  as the profile graph), broken down into commits / PRs / reviews / issues with
  a daily sparkline, via the GraphQL `contributionsCollection` API.
- **Language breakdown** — *pull requests* are classified by the languages of
  their changed files, showing open vs. closed counts per language. Note this is
  PR-based: if your activity is mostly direct commits, this panel will be sparse
  while the Contributions panel shows the real totals.
- **PR scope** — counts PRs each user authored *or was involved in*
  (`involves:` search).
- **AI summaries** — press `s` on a language to have the `claude` CLI explain
  what those PRs were about.

## Setup

1. Copy the example config and fill it in:

   ```sh
   cp config.example.toml config.toml
   ```

   ```toml
   base_url = "https://api.github.com"   # or https://your-host/api/v3 for Enterprise
   token = "ghp_..."                      # or set GITHUB_TOKEN instead
   users = ["octocat", "torvalds"]
   timeline_days = 7
   ```

   The token needs read access to the repositories you want reflected. If
   `token` is omitted, the `GITHUB_TOKEN` environment variable is used.
   `config.toml` is gitignored.

2. Run:

   ```sh
   cargo run                 # uses config.toml, last 7 days
   cargo run -- --days 30    # override the window
   cargo run -- --config other.toml
   ```

## Keys

| Key        | Action                                  |
|------------|-----------------------------------------|
| `↑` / `k`  | Select previous language                |
| `↓` / `j`  | Select next language                    |
| `s`        | Summarize the selected language's PRs with `claude` |
| `r`        | Refresh data                            |
| `q` / `Esc`| Quit                                    |

The `s` feature shells out to the `claude` CLI, which must be on your `PATH`.
