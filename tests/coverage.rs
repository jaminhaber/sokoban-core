//! Level coverage suite — baseline visibility into how many levels the solver
//! handles under `Strategy::Fast` within a per-level wall-clock budget.
//!
//! Each test is `#[ignore]`d so default `cargo test` stays fast. Run with:
//!
//! ```text
//! cargo test --release --test coverage -- --ignored --nocapture
//! ```
//!
//! Override the per-level timeout (default 2000 ms):
//!
//! ```text
//! SOKOBAN_TIMEOUT_MS=10000 cargo test --release --test coverage -- --ignored --nocapture
//! ```
//!
//! Every solution returned by the solver is replayed through
//! `Level::do_actions` and the resulting `Level::is_solved()` must be true. A
//! wrong solution fails the test immediately — solver timeouts and "no
//! solution" verdicts are tallied but do not fail the test (they are baseline
//! measurements).

use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

use sokoban_core::{
    solver::{Solver, Strategy, Terminator},
    Level, SearchError,
};

const DEFAULT_TIMEOUT_MS: u64 = 2_000;

#[derive(Default)]
struct CoverageStats {
    solved: usize,
    timed_out: usize,
    no_solution: usize,
    total_time: Duration,
}

fn per_level_timeout() -> Duration {
    let ms: u64 = std::env::var("SOKOBAN_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TIMEOUT_MS);
    Duration::from_millis(ms)
}

fn run_collection(name: &str, path: impl AsRef<Path>, expected_total: usize) -> CoverageStats {
    let xsb = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "could not read level file {}: {}",
            path.as_ref().display(),
            e
        )
    });

    let timeout = per_level_timeout();
    let mut stats = CoverageStats::default();
    let mut parsed = 0;

    for (idx, parse_result) in Level::load_from_str(&xsb).enumerate() {
        let level_no = idx + 1;
        parsed += 1;

        let mut level = match parse_result {
            Ok(l) => l,
            Err(e) => {
                eprintln!("{:>12} #{:>3}: parse error: {:?}", name, level_no, e);
                continue;
            }
        };

        let solver = Solver::new(level.map().clone(), Strategy::Fast)
            .with_terminator(Terminator::Timeout(timeout));

        let started = Instant::now();
        let result = solver.a_star_search();
        let elapsed = started.elapsed();
        stats.total_time += elapsed;

        match result {
            Ok(actions) => {
                level
                    .do_actions(actions.iter().map(|a| a.direction()))
                    .expect("solver returned actions that the level rejected");
                assert!(
                    level.is_solved(),
                    "{} #{}: replayed solution did not reach a solved state",
                    name,
                    level_no
                );
                stats.solved += 1;
                eprintln!(
                    "{:>12} #{:>3}: ok    pushes={:>4} moves={:>5}  in {:?}",
                    name,
                    level_no,
                    actions.pushes(),
                    actions.moves(),
                    elapsed
                );
            }
            Err(SearchError::Terminated) => {
                stats.timed_out += 1;
                eprintln!("{:>12} #{:>3}: TIMEOUT after {:?}", name, level_no, elapsed);
            }
            Err(SearchError::NoSolution) => {
                stats.no_solution += 1;
                eprintln!(
                    "{:>12} #{:>3}: no-solution after {:?}",
                    name, level_no, elapsed
                );
            }
        }
    }

    assert_eq!(
        parsed, expected_total,
        "{}: expected {} levels, parsed {}",
        name, expected_total, parsed
    );
    stats
}

fn report(name: &str, stats: &CoverageStats, total: usize) {
    eprintln!(
        "\n{:>12}: solved {}/{}  (timeout {}, no-solution {})  wall {:?}\n",
        name, stats.solved, total, stats.timed_out, stats.no_solution, stats.total_time
    );
}

#[test]
#[ignore]
fn coverage_microban() {
    let stats = run_collection("Microban", "assets/Microban_155.xsb", 155);
    report("Microban", &stats, 155);
}

#[test]
#[ignore]
fn coverage_microban_ii() {
    let stats = run_collection("Microban II", "assets/Microban II_135.xsb", 135);
    report("Microban II", &stats, 135);
}

#[test]
#[ignore]
fn coverage_xsokoban() {
    let stats = run_collection("XSokoban", "assets/XSokoban_90.xsb", 90);
    report("XSokoban", &stats, 90);
}
