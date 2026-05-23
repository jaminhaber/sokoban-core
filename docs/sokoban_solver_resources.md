# Sokoban Solver Resources

Useful references for future solver work in this repo.

## Core Solver Techniques

- [Rolling Stone solver notes](https://webdocs.cs.ualberta.ca/~games/Sokoban/program.html)
  - Practical checklist of classic Sokoban search enhancements: IDA*, min-cost matching, transposition tables, move ordering, deadlock tables, tunnel macros, goal macros, relevance cuts, overestimation, and random restarts.
  - Most relevant to this repo: compare the current `Solver` against the Rolling Stone enhancement ladder and decide which high-impact ideas are worth implementing next.

- [Timo Virkkala, Solving Sokoban](https://www.sokoban.dk/wp-content/uploads/2016/02/Timo-Virkkala-Solving-Sokoban-Masters-Thesis.pdf)
  - Broad survey of Sokoban search approaches: A*, IDA*, lower bounds, transposition tables, macro moves, reverse/bidirectional solving, rooms/tunnels/chambers, corrals, deadlocks, and PI-corral pruning.
  - Most relevant to this repo: use as the map for solver design decisions and terminology.

- [Single-Agent Search in the Presence of Deadlocks](https://m.aaai.org/Library/AAAI/1998/aaai98-059.php)
  - Junghanns and Schaeffer paper on pattern search: dynamically identifying minimal deadlock patterns and using them to set heuristic values to infinity.
  - Most relevant to this repo: possible next step after static dead squares, 2x2 checks, freeze checks, and optional corral pruning.

- [Sokoban: improving the search with relevance cuts](https://www.sciencedirect.com/science/article/pii/S0304397500000803)
  - Junghanns and Schaeffer paper on pruning moves that are not locally relevant to recent moves.
  - Most relevant to this repo: candidate branch-factor reduction once correctness and baseline coverage are solid.

- [Optimal Sokoban solving using pattern databases with specific domain knowledge](https://www.sciencedirect.com/science/article/pii/S0004370215000867)
  - Pereira, Ritt, and Buriol paper on instance decomposition and pattern databases for stronger admissible heuristics and deadlock detection.
  - Most relevant to this repo: long-term path for stronger optimal solving beyond min-cost box-goal matching.

## Implementation Notes

- [Sokoban MinimumMovesSolver results](https://computerpuzzle.net/english/sokoban/mms/index.html)
  - Takaken's MinimumMovesSolver publishes proved minimum-move counts for several standard collections. The Microban result table is available as [`microban_mms.txt`](https://computerpuzzle.net/english/sokoban/mms/microban_mms.txt).
  - Most relevant to this repo: use the published counts as regression oracles for `Strategy::OptimalMove` without importing full, license-unclear solution strings.

- [Sokoban Reach and Code Performance](https://timallanwheeler.com/blog/2022/01/23/sokoban-reach-and-code-performance/)
  - Practical notes on state representation, reachability calculation, allocation reduction, and available-push generation.
  - Most relevant to this repo: optimize `reachable_area_with_distances`, player normalization, and successor generation.

- [Push-Permutation Optimization of Sokoban Solutions](https://timallanwheeler.com/blog/2022/02/20/push-permutation-optimization-of-sokoban-solutions/)
  - Describes improving move count by reordering pushes after a valid or push-optimal solution has been found.
  - Most relevant to this repo: possible post-processing pass for `Strategy::Fast` / `Strategy::OptimalPush` results when player-move quality matters.

## Ideas To Revisit

- Deadlock pattern learning or deadlock tables for small local regions.
- Goal-room packing order and goal macros.
- Relevance cuts or another conservative move-pruning scheme.
- Better tunnel/chamber/room analysis, especially for macro generation.
- Incremental or allocation-light reachability for successor generation.
- Richer box waypoint predecessor tables for heavily self-overlapping replay/UI paths.
- Optional solution post-optimizer that improves player moves without changing push count.
- Larger ignored optimality regression tests against the full Takaken Microban table.
