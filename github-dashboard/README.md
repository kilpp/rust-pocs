# github-dashboard

A terminal dashboard (Rust + [ratatui](https://ratatui.rs)) that reports GitHub
contribution activity for a configurable set of users over a configurable
timeline, with a list of the pull requests in the window — and can ask the local
`claude` CLI what a given pull request is about.

## Features

- **Configurable users & endpoint** — track any GitHub usernames; works against
  public GitHub or a GitHub Enterprise server (`base_url`).
- **Configurable timeline** — default last 7 days, overridable with `--days`.
- **Contributions** — GitHub-style contribution totals per user (the same data
  as the profile graph), broken down into commits / PRs / reviews / issues with
  a daily sparkline, via the GraphQL `contributionsCollection` API.
- **Pull request list** — every PR in the window, grouped open-first then
  closed, with its state (open / draft / merged / closed), number, and title.
  Note this is PR-based: if your activity is mostly direct commits, this panel
  will be sparse while the Contributions panel shows the real totals.
- **Review & CI badges** — each PR shows its aggregate review decision
  (`✓` approved, `✗` changes requested, `◷` review required) and a CI rollup
  dot (green passing / red failing / yellow pending). The details pane adds the
  worded status, diff size (`+/-`), and the pending reviewers.
- **"Waiting on you" review queue** — press `w` to filter to the open, ready
  PRs that are awaiting a review from one of your configured users. The header
  always shows the waiting count so you can see the backlog at a glance.
- **PR scope** — lists PRs each user authored *or was involved in*
  (`involves:` search), fetched via GraphQL so the review/CI metadata comes
  back in the same request.
- **AI summaries** — press `s` on a PR to have the `claude` CLI explain what
  that pull request is about.
- **Filter** — `u`/`Tab` cycles the view through each configured user (or all of
  them); combine it with `w` to scope the review queue to a single user.
- **Jump to a PR** — select a PR with `↑`/`↓` and press `o` to open it on GitHub
  (works against Enterprise hosts too).

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

| Key          | Action                                  |
|--------------|-----------------------------------------|
| `↑` / `k`    | Select previous pull request            |
| `↓` / `j`    | Select next pull request                |
| `o` / `Enter`| Open the selected PR in your browser    |
| `u` / `Tab`  | Cycle the user filter (all → each user) |
| `w`          | Toggle the "waiting on you" review queue |
| `s`          | Summarize the selected PR with `claude` |
| `r`          | Refresh data                            |
| `q` / `Esc`  | Quit                                    |

The `s` feature shells out to the `claude` CLI, which must be on your `PATH`.
