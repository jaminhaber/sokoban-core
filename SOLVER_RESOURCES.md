# Sokoban Solver Resources

A curated reading list for writing and improving Sokoban solvers. Organized
roughly from "start here" to "state of the art".

## Start here — community wikis & overviews

The Sokoban Wiki is the single most useful resource. Read these three pages
before anything else; most other links assume you know the vocabulary they
introduce (pushes vs. moves, dead squares, freeze deadlocks, corrals, PI-Corral,
tunnel macros, transposition tables, lower bounds).

- [Sokoban Wiki — Solver](http://sokobano.de/wiki/index.php?title=Solver) — overview of every technique a competitive solver uses.
- [Sokoban Wiki — Deadlocks](http://sokobano.de/wiki/index.php?title=Deadlocks) — taxonomy of deadlock types (dead-square, freeze, corral, bipartite, closed diagonal, …).
- [Sokoban Wiki — How to detect deadlocks](http://sokobano.de/wiki/index.php?title=How_to_detect_deadlocks) — concrete algorithms for each deadlock category.
- [Sokoban Wiki — Solver Statistics](http://sokobano.de/wiki/index.php?title=Solver_Statistics) — head-to-head benchmark numbers across the major solvers; useful as a target.
- [Sokoban.dk — Solvers and Optimizers](https://sokoban.dk/solvers-and-optimizers/) — historical narrative of how each solver advanced the state of the art.

## Tutorials & blog posts

These walk through a working implementation end-to-end. Good for cross-checking
your own design decisions.

- [Tim Wheeler — Basic Search Algorithms on Sokoban](https://timallanwheeler.com/blog/2022/01/19/basic-search-algorithms-on-sokoban/) — clearest writeup of why you search push-space, not move-space, and how to layer BFS / A* / IDA* on top.
- [Pavel Klavík — Sokoban Solver (short documentation, PDF)](https://pavel.klavik.cz/projekty/solver/solver.pdf) — concise tour of lower bounds, transposition tables, and pruning, written by the author of a working solver.
- [Brian Damgaard — YASS "scribbles"](http://sokobano.de/wiki/index.php?title=Sokoban_solver_%22scribbles%22_by_Brian_Damgaard_about_the_YASS_solver) — implementer notes from the author of YASS; lots of practical detail you won't find in papers.
- [Rosetta Code — Sokoban](https://rosettacode.org/wiki/Sokoban) — minimal solvers in many languages; useful for sanity-checking a basic BFS.

## Foundational papers

Read in roughly this order. The Junghanns & Schaeffer paper is *the* canonical
reference — most ideas in modern solvers trace back to it.

- [Junghanns & Schaeffer — Sokoban: A Challenging Single-Agent Search Problem (PDF)](https://webdocs.cs.ualberta.ca/~jonathan/publications/ai_publications/soko.pdf) — Rolling Stone solver. Introduces deadlock tables, tunnel/goal macros, relevance cuts, and the push-search formulation. Required reading.
- [Virkkala — Solving Sokoban (Master's thesis, 2011, PDF)](http://sokoban.dk/wp-content/uploads/2016/02/Timo-Virkkala-Solving-Sokoban-Masters-Thesis.pdf) — best self-contained survey of all the classical techniques in one document.
- [Takes — Sokoban: Reversed Solving (PDF)](https://liacs.leidenuniv.nl/~takesfw/pdf/sokoban.pdf) — backward search from goal states; useful complement to forward search.
- [Botea, Müller, Schaeffer — Using Abstraction for Planning in Sokoban](https://link.springer.com/chapter/10.1007/3-540-61532-6_50) — hierarchical decomposition (the "rooms and tunnels" idea).

## Heuristics & lower bounds

A* / IDA* lives or dies by its heuristic. The minimum-cost bipartite matching
on push-distances is the standard baseline; the IJCAI 2016 paper is the modern
reference for tightening it.

- [Improved Heuristic and Tie-Breaking for Optimally Solving Sokoban (Holte et al., IJCAI 2016, PDF)](https://webdocs.cs.ualberta.ca/~holte/Publications/ijcai2016-sokoban.pdf) — better admissible heuristics and tie-breaking for optimal solving.
- [Optimal Sokoban Solving Using Pattern Databases with Specific Domain Knowledge (Pereira et al.)](https://www.sciencedirect.com/science/article/pii/S0004370215000867) — pattern databases as both heuristic and deadlock detector.
- [Solving Sokoban Optimally Using Pattern Databases for Deadlock Detection (PDF)](https://www.inf.ufrgs.br/~mrpritt/Publications/P52-enia2014.pdf) — the same idea applied specifically to deadlock pruning.
- [Learning Deadlocks in Sokoban (PDF)](https://lume.ufrgs.br/bitstream/handle/10183/190174/001088740.pdf?sequence=1) — neural net for deadlock detection; sketches what's possible beyond hand-coded rules.

## State of the art — Festival & FESS

Festival is the first solver to clear all 90 XSokoban levels. The FESS algorithm
(Feature Space Search) it's built on is genuinely different from A*/IDA* and
worth understanding separately.

- [Festival Sokoban Solver — official site](https://festival-solver.site/) — papers, downloads, and detailed write-ups.
- [Festival source code (Android port, GitHub)](https://github.com/joriswit/Festival) — the only public source for Festival; original C++ was distributed via the Sokoban mailing list.
- [Festival source code thread on groups.io](https://groups.io/g/sokoban/topic/festival_source_code/102750985) — discussion of the source release.
- [JSoko — announcement of Festival 2.4](https://www.sokoban-online.de/2022/10/15/festival-sokoban-solver-version-2-4-has-been-published/)
- [JSoko — Festival papers announcement](https://www.sokoban-online.de/2020/08/22/the-papers-for-the-festival-sokoban-solver-have-been-published/) — links to the FESS / CoG 2020 papers.
- [Solving Sokoban with Forward-Backward Reinforcement Learning (PDF)](https://ojs.aaai.org/index.php/SOCS/article/download/18580/18369/22099) — solves 88/90 XSokoban levels; second only to Festival among published methods.

## Machine learning approaches

Useful if you want to go beyond classical search. Sokoban is PSPACE-complete,
so RL alone struggles; the curriculum-learning paper is the strongest ML result
on hard hand-designed levels.

- [A Novel Automated Curriculum Strategy to Solve Hard Sokoban Planning Instances (NeurIPS 2020)](https://proceedings.neurips.cc/paper/2020/hash/2051bd70fc110a2208bdbd4a743e7f79-Abstract.html) ([arXiv](https://arxiv.org/abs/2110.00898), [Cornell PDF](https://www.cs.cornell.edu/gomes/pdf/2021_feng_arxiv_curriculum.pdf)) — RL agent that solves levels classical search can't reach in years.
- [Single-Player MCTS Performance in Sokoban (Expert Systems with Applications)](https://www.sciencedirect.com/science/article/abs/pii/S0957417421015372) — what MCTS can and can't do here.
- [AI in Game Playing: Sokoban Solver (arXiv 1807.00049)](https://arxiv.org/abs/1807.00049) — classical-vs-learning comparison on a benchmark suite.
- [Solving Sokoban Efficiently: Search Tree Pruning (Bachelor thesis, PDF)](https://doc.neuro.tu-berlin.de/bachelor/2023-BA-NiklasPeters.pdf) — recent student work combining pruning techniques.

## Reference solvers (read the source)

When stuck, read existing implementations. Listed roughly best → simplest.

- [Festival (Android port)](https://github.com/joriswit/Festival) — state-of-the-art, FESS-based. Dense but the algorithm of record.
- [dangarfield/sokoban-solver](https://github.com/dangarfield/sokoban-solver) — Rust port of Festival compiled to WASM; closer to idiomatic Rust than the C++ original.
- [JSoko](https://jsokoapplet.sourceforge.io/) — open-source Java solver/player; the wiki documentation effectively describes JSoko's internals.
- [margaritageleta/sokoban](https://github.com/margaritageleta/sokoban) — A* + RL hybrid with a clear writeup of pseudocode and complexity.
- [xbandrade/sokoban-solver-generator](https://github.com/xbandrade/sokoban-solver-generator) — readable BFS / A* / Dijkstra in Python; checks deadlocks before enqueue.
- [stepzhou/sokoban-solver](https://github.com/stepzhou/sokoban-solver) — BFS / DFS / UCS / Greedy with multiple heuristics; good for ablation testing.
- [shray-sharma/sokoban-solver](https://github.com/shray-sharma/sokoban-solver) — Anytime GBFS and Weighted A*.
- [andrej/sokoban](https://github.com/andrej/sokoban) — A* with a smart heuristic; small, easy to read end-to-end.
- [GitHub topic: sokoban-solver](https://github.com/topics/sokoban-solver) — browse the long tail.

## Benchmarks & test levels

Almost every paper above reports results on XSokoban (the 90-level set) and
Microban. Use the same sets so your numbers are comparable.

- [Sokoban Wiki — Solver Statistics](http://sokobano.de/wiki/index.php?title=Solver_Statistics) — per-level results for every major solver. The de-facto leaderboard.
- [Sokoban.dk](https://sokoban.dk/) — DrFogh's pages; level collections and historical benchmark posts.

## Suggested reading order if you're starting from scratch

1. Sokoban Wiki — Solver, Deadlocks, How to detect deadlocks (skim all three first).
2. Tim Wheeler's blog post — get a working push-space BFS in your head.
3. Junghanns & Schaeffer — Rolling Stone paper for the canonical technique set.
4. Virkkala's thesis — fills in everything the wiki and the paper gloss over.
5. Pick one reference solver and read it cover to cover.
6. IJCAI 2016 heuristic paper, then the Festival/FESS papers when you want to push past the classical ceiling.
