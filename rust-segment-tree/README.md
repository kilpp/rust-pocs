# Rust Segment Tree

A segment tree implementation in Rust with two binaries: a tree visualizer and a DPS tracker demo.

---

## What is a Segment Tree?

A segment tree is a data structure that stores an array and answers **range queries** efficiently.

Given an array of `n` elements, a segment tree lets you:
- **Query** the sum of any range `[l, r)` in **O(log n)**
- **Update** any single element in **O(log n)**

Without it, a range sum requires O(n) time (iterate and add). With it, you pay O(n) once to build, then every query and update is O(log n).

### How it works

The tree is stored as a flat array of size `2n`. The leaves (indices `n..2n`) hold the input values. Each internal node (indices `1..n`) holds the sum of its two children.

```
Input: [3, 1, 4, 1, 5, 9, 2, 6]

                  31          <- tree[1]  (sum of everything)
          10              21
      4       6       14      7
    3   1   4   1   5   9   2   6
```

**Query `[2, 6)`** — climb from both ends, collecting nodes that fall entirely within the range:

```
start = 2 + n = 10  →  collect tree[10]=4, tree[11]=1, tree[12]=5, tree[13]=9  = 19
```

**Update index 3 to value 7** — overwrite the leaf, then recompute parents up to root.

### Implementation

```rust
pub struct SegmentTree {
    n: usize,
    tree: Vec<i32>,
}
```

- `tree[n..2n]` — the leaves (input values)
- `tree[i] = tree[2i] + tree[2i+1]` — each parent is the sum of its children
- Update: set leaf, then walk up with `i >>= 1`
- Query: walk from both ends toward the middle, consuming edge nodes

---

## Binaries

### `tui` — Interactive Tree Visualizer

```
cargo run --bin tui
```

Renders the segment tree as an ASCII diagram. Highlighted nodes show which ones were touched by the last `update` (red) or `query` (green).

Commands: `update <index> <value>`, `query <l> <r>`, `reset`, `quit`.

---

### `dps-tracker` — DPS Meter Demo

```
cargo run --bin dps-tracker
```

A WoW-style DPS tracker with 5 classes fighting a raid boss.

Each player's damage is stored tick-by-tick in their own segment tree. Rolling DPS is computed with a single range query over the last 4 seconds of ticks — no looping over raw history.

```
DPS = tree.query(tick - 16, tick) / 4.0 seconds
```

**Classes:** Sorcerer · Rogue · Archer · Paladin · Priest

**Controls:** `Space` pause · `R` reset · `Q` quit

![layout]
```
┌─ DPS Meter ──────────────────────────────────────────────┐
│  Sorcerer  ████████████████████████  7.2k  100%          │
│  Rogue     ████████████████████      6.1k   85%          │
│  Archer    ██████████████████        5.8k   81%          │
│  Paladin   ██████████████            4.4k   61%          │
│  Priest    ████████                  2.4k   34%          │
├─ DPS Timeline (last 30 s) ───────────────────────────────┤
│  ⠀⠀⠀⠀⡠⠔⠒⠒⠤⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⠔⠒⠒⠢⠤⡀⠀⠀⠀⠀⠀⠀⠀⠀  │
│  ⠀⠀⡠⠊⠀⠀⠀⠀⠀⠈⠢⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⠔⠁⠀⠀⠀⠀⠀⠈⠢⡀⠀⠀⠀⠀⠀⠀  │
├─ Status ─────────────────────────────────────────────────┤
│  Fight: 0:42  |  Raid DPS: 25.9k  |  [Space]=pause ...  │
└──────────────────────────────────────────────────────────┘
```
