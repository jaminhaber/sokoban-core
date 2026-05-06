# Solver Plan — From Working to Cutting Edge

A roadmap for evolving `sokoban_core` from a competent A*/IDA* solver into one
that can stand alongside Festival, Sokolution, and YASS. Phases are ordered
by impact ÷ effort. Anything in P0 is a correctness fix and should land first.

The plan starts with audit findings (what's there now, with file:line refs),
then prescribes work in five phases plus a benchmark harness.

---

## 1. Audit — current state

### What's good

- Push-space search with optional move-cost edge weights (`solver.rs:36-55`, `node.rs:38-69`).
- Three strategies (`Fast`, `OptimalPush`, `OptimalMove`) sharing one successor generator (`solver.rs:14-27`, `node.rs:83-161`).
- Player canonicalization for push-space search (`state.rs:74-115`, `node.rs:42-44`).
- A* with reopening + IDA* with path-only cycle check (`solver.rs:203-377`).
- Lazy precomputation of push-distance abstraction and tunnel macros (`solver.rs:46-53`, `solver.rs:401-529`).
- Min-cost bipartite-matching heuristic via subset DP — admissible, robust up to ~22 boxes (`matching.rs`, `state.rs:44-71`).
- `BoxSet`: 512-byte fixed bitset for box configurations, deterministic hashing, fast iteration (`box_set.rs`).
- Static deadlock + freeze deadlock + useless-floor pruning (`deadlock.rs`).
- Push-distance reverse BFS gives implicit dead-square set (`solver.rs:410-469`).
- Solution reconstruction re-simulates from start to guarantee validity given push-space canonicalization (`solver.rs:531-628`).
- Terminator: timeout / iteration cap (`solver.rs:58-104`).
- Tunnel macro detector (`solver.rs:476-529`) and gated successor compression (`node.rs:115-130`).

### What's missing or weak

| Area | Observation | File:line |
|---|---|---|
| Hash keys | A*'s open/closed state is keyed on a `u64` SipHash. With billions of states the collision probability becomes nonzero, and a collision silently corrupts results. | `solver.rs:206-251`, `state.rs:85-115` |
| Freeze deadlock | The "exempt if on goal" check is applied only to the freshly pushed box, not to the whole frozen connected component. Pushing box A onto a goal can freeze a neighbor B that is off-goal — currently undetected. | `node.rs:137-146`, `deadlock.rs:52-94` |
| Heuristic | Min-cost matching uses single-box push distances on an empty board. It ignores that other boxes block paths, so it underestimates by a lot on packed levels. | `state.rs:44-71`, `solver.rs:410-469` |
| Heuristic for `OptimalMove` | `h` returns push count, used as a lower bound on moves. Admissible but very loose — does not lower-bound walking cost between pushes. | `node.rs:50-60` |
| Deadlock coverage | No PI-corral, no closed-diagonal (2×2 box block), no bipartite goal/box reachability, no pattern-database deadlocks. These are responsible for most of Sokolution/Festival's pruning. | `deadlock.rs` |
| Goal ordering / packing | No reverse pull search, no goal-room analysis, no packing order. State of the art relies heavily on these. | (absent) |
| Macros | Only tunnel macros. No goal macros (push straight to goal through clean corridor), no room macros. | `solver.rs:476-529` |
| Successor ordering | Successors are returned in box-iteration order; no h-based ordering. Hurts both A* (priority queue churn) and IDA* (worse pruning). | `node.rs:83-161` |
| Heuristic recomputation | Heuristic recomputed in full at every successor; only one box moved, so most of the matching is reusable. | `node.rs:38-46`, `state.rs:44-71` |
| Matching cap | Subset DP returns `None` for `n > 22`. Many real levels (some XSokoban, MasMicroban) exceed this. No Hungarian fallback. | `matching.rs:27-31` |
| IDA* TT | IDA* uses a path-only `HashSet`; no iteration-spanning transposition table or cost cache. | `solver.rs:280-377` |
| Reachability cost | Each successor expansion runs a fresh BFS for player reachability and distances. Could share between sibling expansions. | `node.rs:86-89` |
| Hashing | `DefaultHasher` (SipHash) is overkill for the inner loop; `ahash`/`FxHash` is 2–4× faster on these payloads. | `state.rs:88-115` |
| Static deadlock set | `calculate_static_deadlocks` (`deadlock.rs:101-150`) is computed but never used by the solver — `lower_bounds.contains_key` is the only dead-square gate. They overlap but neither is strictly stronger. | `deadlock.rs:101-150`, `solver.rs:107-109` |
| Tests | Solver tests cover 3 BoxWorld levels + 3 ad-hoc puzzles. No Microban / XSokoban regression suite, no push-optimality assertion, no move-optimality assertion, no determinism check across runs. | `tests/solver.rs` |
| Bench | Two benchmarks (one mega-level, one BoxWorld). IDA* benchmark commented out. No suite-level "levels solved within N seconds" metric. | `benches/benches/solver.rs` |

---

## 2. Correctness fixes (P0 — land first)

These are bugs, not improvements. They unblock everything downstream.

### P0.1 — Frozen-component goal check

**Bug.** In `node.rs:137-146`, the freeze deadlock check exempts the pushed box if it lands on a goal. The Sokoban Wiki rule is stricter: a frozen group is OK *only if every box in the connected frozen component is on a goal*. The current `is_freeze_deadlock` (`deadlock.rs:52-94`) recurses through neighbor boxes and returns true for the whole group, but the call site only checks the pushed box's goal status.

**Fix.** Replace the call site with a helper that:
1. Runs `is_freeze_deadlock` on the pushed box.
2. If frozen, computes the connected component of mutually-frozen boxes.
3. Returns "deadlock" iff at least one box in that component is off-goal.

Add a regression test with a hand-built XSB level that triggers the case.

### P0.2 — Collision-safe state keys

**Bug.** `solver.rs:206-211` keys the open/closed structures on a `u64`. With sufficiently large state spaces, two different `(player, BoxSet)` states can hash to the same `u64`, producing wrong best-known costs and (worse) wrong reconstruction.

**Fix.** Replace `HashMap<u64, …>` with `HashMap<StateKey, …>` where `StateKey = (i32, BoxSet)` for `OptimalMove` and `(IVector2, BoxSet)` after canonicalization for push-space modes. `BoxSet` is `Eq + Hash` already. Keep a `u64` cache in `Node` for fast priority-queue comparisons but never use it as a map key.

### P0.3 — Solver determinism test

Hash iteration order can leak into the solution path. Add a test that runs the same level twice and asserts the solution is byte-identical (or at least the same length). Catches future regressions where someone introduces ambient randomness.

---

## 3. Phase 1 — Heuristic & deadlock improvements (largest immediate gains)

Roughly: each item below is the difference between "solves Microban-1..120" and "solves Microban-1..150." The deadlock work alone typically halves expanded nodes on hard levels.

### P1.1 — Tighter dead-square set

Combine three sources into a single `dead_squares: HashSet<IVector2>`:
- `lower_bounds.contains_key` complement (no path to any goal in abstraction).
- Corner + groove from `calculate_static_deadlocks`.
- Squares where pushing the box creates an immediate freeze with the surrounding walls (precomputed once).

Use this set as a fast O(1) gate before the full freeze check.

### P1.2 — Closed-diagonal (2×2) deadlock

Pre-pruning rule: if four cells form a 2×2 block in which every cell is wall-or-box and at least one box is off-goal, the configuration is dead. Cheap and very effective. Add unit tests with all rotations/reflections.

### P1.3 — Bipartite reachability deadlock

After each push, build a bipartite graph (boxes × goals) with an edge when the box can still reach that goal in the abstraction *given current box positions* (not just the empty-board abstraction). If the graph has no perfect matching, the state is dead. Reuses the matching code from `matching.rs`.

### P1.4 — PI-corral pruning

The single highest-impact addition for hard levels. A "corral" is a region the player cannot enter; a PI-corral is one whose interior boxes can be analyzed in isolation. If no push from the corral boundary can resolve it, the state is a deadlock. Implement per Junghanns & Schaeffer §5 (see `SOLVER_RESOURCES.md`).

### P1.5 — Reverse BFS for goal positions and packing order

Run BFS *backwards* from goal cells using *pull* moves to compute, for each goal, its reachable set and a "packing order" — the order in which goals must be filled so that future packings don't push earlier boxes off their goals. This is the foundation Festival builds its FESS features on.

Expose `Solver::packing_order() -> Vec<IVector2>` and use it as a tie-breaker in `Fast`.

### P1.6 — Tighter heuristic — incremental matching

Per push, only the moved box's row of the cost matrix changes. Use Hungarian-style label updates (or just recompute one row + an O(n²) repair) instead of full subset DP. Brings heuristic cost from O(n·2ⁿ) per state to amortized O(n²).

### P1.7 — Heuristic that respects current box positions

Replace the empty-board push distance with a per-state push distance that treats other boxes as walls. Too expensive to compute fresh per state; instead memoize per (frozen-subset, box-of-interest). Pragmatic compromise: use empty-board distance + a small correction term equal to the count of boxes between source and goal on the manhattan path.

### P1.8 — Move-optimal heuristic improvement

For `OptimalMove`, add a walking lower bound: for each pushed-box-to-goal pair, add the player's minimum walk to reach the behind-square at least once. Cheap admissible improvement, big help in move-optimal mode.

---

## 4. Phase 2 — Search improvements

### P2.1 — Successor ordering

Sort successors ascending by `priority` before iterating in IDA* and by `h` in A* (matters for cache locality and IDA* pruning).

### P2.2 — IDA* transposition table

Add a bounded LRU table mapping state-key → best `f` seen this iteration; skip nodes whose `f` is ≥ recorded value. Also consider IDA*+TT (single global table across iterations with cost validation).

### P2.3 — Goal macros

Detect, per goal, the set of cells from which a box can be pushed straight to that goal through a corridor whose interior contains no decision points. When successor generation sees such a push, expand it as a single macro step.

### P2.4 — Online deadlock cache

When a freeze/PI-corral/2×2 fires, hash the relevant local box pattern and cache it. Future expansions test the cache first. Bound size to keep memory predictable.

### P2.5 — Hungarian / min-cost flow fallback

Implement a Hungarian-algorithm path in `matching.rs` for `n > 22`. Subset DP stays as the default for small `n` (often faster in practice).

### P2.6 — Faster hashing

Switch `HashMap` / `HashSet` in the search hot path from `DefaultHasher` (SipHash-1-3) to `ahash` or `FxHash` behind a feature flag. Keep SipHash for any user-facing API where adversarial inputs matter.

### P2.7 — Greedy best-first variant

Add `Strategy::Greedy` (pure h, no g) for an even faster non-optimal mode. Combined with online deadlock cache it often blows through huge levels in seconds.

### P2.8 — Bidirectional search

Run a forward push search from start and a backward pull search from each goal configuration. Meet in the middle. Especially strong for "highway" levels with long forced corridors.

---

## 5. Phase 3 — Cutting-edge algorithms

### P3.1 — FESS (Feature Space Search)

Festival's signature algorithm. Search nodes are projected into a low-dimensional *feature space* (e.g., `(packed_goals, connectivity, room_count)`); search proceeds in feature space, choosing the next *cell* to expand rather than the next state. This decouples search progress from raw state count and is the core reason Festival solves all 90 XSokoban levels.

Implementation outline:
1. Define 3–5 features mirroring Festival's set.
2. Maintain a per-feature-cell bucket of states.
3. At each step, pick the cell with the fewest expansions so far, expand its best state.

This is a substantial project (multi-week) but brings the largest expected jump in level coverage.

### P3.2 — Pattern database heuristic

Pre-solve disjoint sub-puzzles (e.g., all goals in one room) and store push-cost minimums keyed on the sub-puzzle's box configuration. At search time, sum sub-puzzle costs as the heuristic. Optimal-quality pruning for levels that decompose cleanly.

### P3.3 — Forward-Backward RL hybrid (optional, frontier)

The 2022 SOCS paper "Solving Sokoban with Forward-Backward Reinforcement Learning" places second to Festival on XSokoban. Train a policy network on a curriculum (per Feng et al. 2020) and use it to propose successors in `Fast` mode. Keep classical search as the truth oracle.

### P3.4 — DFPN / iterative AND-OR search

Useful for the hardest hand-crafted instances where A*/IDA* both blow up. Lower priority than FESS but a documented research direction.

### P3.5 — Rolling-Stone-style relevance cuts

Junghanns & Schaeffer §6: only consider pushes that affect goals not yet "abstractly satisfied." Fast and very pruney once goal-rooms are identified (P1.5).

---

## 6. Phase 4 — Engineering & performance

### P4.1 — Reachability sharing

Successor generation does one BFS per node (`node.rs:86-89`). For a single A* expansion, this is fine; for IDA* re-expanding the same state across iterations, cache by state-key.

### P4.2 — Node arena

Replace per-node `HashMap` lookups for `parent` / `states` (`solver.rs:208-219`) with a `Vec<Node>` arena indexed by `u32`. Halves memory and improves cache locality.

### P4.3 — Packed BoxSet

`BoxSet` reserves 4096 bits unconditionally (`box_set.rs:8-10`). For levels with W·H ≪ 4096 this wastes hash work. Add a compile-time generic over bit-count, or use `[u64; (W*H + 63)/64]` chosen at solver-construction time.

### P4.4 — SIMD bit ops on `BoxSet`

Difference, union, popcount on the 64-word representation are obvious AVX2/NEON candidates. Validate with the existing `test_hash_deterministic` first.

### P4.5 — Parallel search (HDA*)

Hash-Distributed A*: shard states by hash across N worker threads. Embarrassingly parallel for compute-bound hard levels. Best implemented after P4.2 (arena) and P2.6 (faster hash).

### P4.6 — Profile-guided optimization

A `cargo pgo` pass on the criterion suite typically buys 5–15% with no code changes.

---

## 7. Phase 5 — Test & benchmark harness

This is independent of the algorithmic phases and should land in parallel with P0/P1.

### P5.1 — Level coverage suite

Add `tests/coverage.rs` (gated behind `#[ignore]` or a `coverage` feature so default `cargo test` stays fast):

- Microban (155 levels) — every level should solve in < 5 s wall-clock with `Strategy::Fast`.
- Microban II (135 levels) — target ≥ 130 within 30 s each.
- XSokoban (90 levels) — target progressively higher counts as phases land. Initial baseline first.
- For each level, assert solution validity by replaying through `Level::do_actions` and checking `is_solved`.
- Persist a snapshot file `tests/coverage_snapshot.json` with `{level_id, pushes, moves, time_ms}` so regressions are visible in `git diff`.

### P5.2 — Push-/move-optimality tests

Hand-pick 5–10 small levels with known optimal push and move counts (Microban has many that take ≤30 pushes). Add tests:
- `OptimalPush` matches known optimum.
- `OptimalMove` matches known optimum.
- `Fast` ≤ 1.5 × `OptimalPush` on those levels.

### P5.3 — Solver criterion harness

Re-enable IDA* benchmarks (`benches/benches/solver.rs:32-50`). Add criterion benches for:
- `lower_bounds()` precompute on representative levels.
- `tunnels()` (already there).
- `Fast` end-to-end on Microban-1, Microban-50, BoxWorld-3, XSokoban-1.
- `heuristic()` per-state cost (synthetic state with N boxes).

### P5.4 — Determinism + property tests

- Same level solved twice → same `Actions`. Done in P0.3.
- Add `proptest` with random small (≤6×6, ≤3 boxes) random levels: any solution returned must replay to a solved state.

### P5.5 — Fuzz the parser

Parser is a common crash vector. Add a `cargo fuzz` target on `Map::from_str`. Cheap and catches edge cases.

---

## 8. Suggested execution order

| Order | Item | Estimated effort | Expected win |
|---:|---|---|---|
| 1 | P0.1 frozen-component goal check | 0.5 day | correctness |
| 2 | P0.2 collision-safe state keys | 1 day | correctness |
| 3 | P5.1 level coverage harness (baseline) | 1 day | visibility |
| 4 | P0.3 determinism test | 0.5 day | correctness |
| 5 | P1.2 closed-diagonal deadlock | 0.5 day | medium |
| 6 | P1.1 unified dead-square set | 0.5 day | small |
| 7 | P1.6 incremental matching | 1–2 days | medium |
| 8 | P2.1 successor ordering | 0.5 day | small-medium |
| 9 | P1.4 PI-corral | 3–5 days | **large** |
| 10 | P1.5 reverse BFS + packing order | 2–3 days | large (enables more) |
| 11 | P2.6 faster hashing | 0.5 day | small |
| 12 | P2.4 online deadlock cache | 1–2 days | medium |
| 13 | P2.7 greedy mode | 0.5 day | small |
| 14 | P1.3 bipartite reachability | 1 day | medium |
| 15 | P2.3 goal macros | 2–3 days | medium |
| 16 | P2.5 Hungarian fallback | 1 day | unblocks big levels |
| 17 | P2.2 IDA* TT | 1–2 days | medium |
| 18 | P4.1 / P4.2 arena + reachability share | 2 days | small-medium |
| 19 | P3.1 FESS | 2–4 weeks | **largest** |
| 20 | P3.2 pattern DBs | 1–2 weeks | medium-large |
| later | P3.3 RL hybrid, P4.5 HDA*, P3.4 DFPN | open-ended | research |

After items 1–10 the solver should be in a comfortably better place than YASS-2018-era; after 19 it is in the same conversation as Festival.

---

## 9. Definition of done — "cutting edge"

A reasonable bar to claim parity with the published state of the art:

- Solves ≥ 89/90 XSokoban levels within 10 minutes each on a modern desktop.
- Solves all 155 Microban + all 135 Microban II within 1 second each.
- Push-optimal results match published optima on the small-level catalogues.
- A reproducible criterion benchmark and a snapshot of `(level, pushes, moves, time)` checked in.
- Public API stable: `Solver::new` / `Strategy` / `Terminator` unchanged; internal swaps don't leak through.

If we hit those four bullets, this library is honest competition for Sokolution; FESS (P3.1) gets us within range of Festival.
