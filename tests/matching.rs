//! Tests for the minimum-cost bipartite matching used as the solver's
//! admissible heuristic.

use sokoban_core::matching::min_cost_matching;

const INF: i32 = i32::MAX;

#[test]
fn empty_matrix_is_zero_cost() {
    let cost: Vec<i32> = Vec::new();
    let (c, assignment) = min_cost_matching(&cost, 0).unwrap();
    assert_eq!(c, 0);
    assert!(assignment.is_empty());
}

#[test]
fn single_element_matches_to_itself() {
    let cost = [7];
    let (c, assignment) = min_cost_matching(&cost, 1).unwrap();
    assert_eq!(c, 7);
    assert_eq!(assignment, vec![0]);
}

#[test]
fn picks_minimum_assignment_two_by_two() {
    // Row-major:
    //   box 0: [1, 4]
    //   box 1: [2, 3]
    // Optimal greedy: box0→goal0 + box1→goal1 = 4.
    let cost = [1, 4, 2, 3];
    let (c, assignment) = min_cost_matching(&cost, 2).unwrap();
    assert_eq!(c, 4);
    assert_eq!(assignment, vec![0, 1]);
}

#[test]
fn matching_is_assignment_constrained_not_greedy() {
    // Greedy would have both boxes pick goal 0 (cost 1 each).
    // The matching must respect 1-to-1 assignment.
    let cost = [1, 100, 1, 100];
    let (c, assignment) = min_cost_matching(&cost, 2).unwrap();
    assert_eq!(c, 1 + 100);
    let mut seen = assignment.clone();
    seen.sort();
    assert_eq!(seen, vec![0, 1]);
}

#[test]
fn impossible_assignment_returns_none() {
    // Both boxes can only reach goal 0 → no perfect match.
    let cost = [5, INF, 3, INF];
    assert!(min_cost_matching(&cost, 2).is_none());
}

#[test]
fn impossible_box_returns_none() {
    // Box 1 can reach no goal at all.
    let cost = [1, 2, INF, INF];
    assert!(min_cost_matching(&cost, 2).is_none());
}

#[test]
fn wrong_length_returns_none() {
    // 6 entries with n = 2 → cost.len() != n*n.
    let cost = [1, 2, 3, 4, 5, 6];
    assert!(min_cost_matching(&cost, 2).is_none());
}

#[test]
fn three_by_three_finds_global_optimum() {
    // Construct a case where the greedy column-by-column choice (1, 1, 5 = 7)
    // is worse than the optimum (3, 1, 1 = 5).
    let cost = [3, 9, 9, 9, 1, 9, 9, 9, 1];
    let (c, assignment) = min_cost_matching(&cost, 3).unwrap();
    assert_eq!(c, 5);
    assert_eq!(assignment, vec![0, 1, 2]);
}

#[test]
fn assignment_is_a_valid_permutation() {
    // Smoke test: every returned assignment must be a permutation of 0..n.
    let cost = [
        4, 1, 3, 2, // box 0
        2, 3, 4, 1, // box 1
        1, 2, 1, 3, // box 2
        3, 4, 2, 1, // box 3
    ];
    let (_, assignment) = min_cost_matching(&cost, 4).unwrap();
    let mut sorted = assignment.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1, 2, 3]);
}

#[test]
fn returns_none_above_size_limit() {
    // The subset-DP implementation guards against `n > 22` to avoid OOM.
    let n = 23;
    let cost = vec![0i32; n * n];
    assert!(min_cost_matching(&cost, n).is_none());
}

#[test]
fn handles_zero_cost_matrix() {
    let n = 4;
    let cost = vec![0i32; n * n];
    let (c, assignment) = min_cost_matching(&cost, n).unwrap();
    assert_eq!(c, 0);
    let mut sorted = assignment.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1, 2, 3]);
}
