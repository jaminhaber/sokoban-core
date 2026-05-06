//! Tests for the minimum-cost bipartite matching used as the solver's
//! admissible heuristic.

use sokoban_core::matching::min_cost_matching;

const INF: i32 = i32::MAX;

#[test]
fn empty_matrix_is_zero_cost() {
    let cost: Vec<Vec<i32>> = Vec::new();
    let (c, assignment) = min_cost_matching(&cost).unwrap();
    assert_eq!(c, 0);
    assert!(assignment.is_empty());
}

#[test]
fn single_element_matches_to_itself() {
    let cost = vec![vec![7]];
    let (c, assignment) = min_cost_matching(&cost).unwrap();
    assert_eq!(c, 7);
    assert_eq!(assignment, vec![0]);
}

#[test]
fn picks_minimum_assignment_two_by_two() {
    // Box 0 → goal 0 costs 1, → goal 1 costs 4
    // Box 1 → goal 0 costs 2, → goal 1 costs 3
    // Optimal: box0→goal1 (4) + box1→goal0 (2) = 6  vs  box0→goal0 (1) + box1→goal1 (3) = 4
    // The greedy choice of box0→goal0 gives 4, which is also optimal.
    let cost = vec![vec![1, 4], vec![2, 3]];
    let (c, assignment) = min_cost_matching(&cost).unwrap();
    assert_eq!(c, 4);
    assert_eq!(assignment, vec![0, 1]);
}

#[test]
fn matching_is_assignment_constrained_not_greedy() {
    // Greedy would have both boxes pick goal 0 (cost 1 each).
    // The matching must respect 1-to-1 assignment.
    let cost = vec![vec![1, 100], vec![1, 100]];
    let (c, assignment) = min_cost_matching(&cost).unwrap();
    assert_eq!(c, 1 + 100);
    // Either assignment is valid as long as it's a permutation.
    let mut seen = assignment.clone();
    seen.sort();
    assert_eq!(seen, vec![0, 1]);
}

#[test]
fn impossible_assignment_returns_none() {
    // Box 0 can only reach goal 0. Box 1 can only reach goal 0. No perfect match.
    let cost = vec![vec![5, INF], vec![3, INF]];
    assert!(min_cost_matching(&cost).is_none());
}

#[test]
fn impossible_box_returns_none() {
    // Box 1 can reach no goal at all.
    let cost = vec![vec![1, 2], vec![INF, INF]];
    assert!(min_cost_matching(&cost).is_none());
}

#[test]
fn non_square_returns_none() {
    let cost = vec![vec![1, 2, 3], vec![4, 5, 6]];
    assert!(min_cost_matching(&cost).is_none());
}

#[test]
fn ragged_rows_return_none() {
    let cost = vec![vec![1, 2], vec![3]];
    assert!(min_cost_matching(&cost).is_none());
}

#[test]
fn three_by_three_finds_global_optimum() {
    // Construct a case where the greedy column-by-column choice (1, 1, 5 = 7)
    // is worse than the optimum (3, 1, 1 = 5).
    let cost = vec![
        vec![3, 9, 9],
        vec![9, 1, 9],
        vec![9, 9, 1],
    ];
    let (c, assignment) = min_cost_matching(&cost).unwrap();
    assert_eq!(c, 5);
    assert_eq!(assignment, vec![0, 1, 2]);
}

#[test]
fn assignment_is_a_valid_permutation() {
    // Smoke test: every returned assignment must be a permutation of 0..n.
    let cost = vec![
        vec![4, 1, 3, 2],
        vec![2, 3, 4, 1],
        vec![1, 2, 1, 3],
        vec![3, 4, 2, 1],
    ];
    let (_, assignment) = min_cost_matching(&cost).unwrap();
    let mut sorted = assignment.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1, 2, 3]);
}

#[test]
fn returns_none_above_size_limit() {
    // The subset-DP implementation guards against `n > 22` to avoid OOM.
    let n = 23;
    let cost: Vec<Vec<i32>> = (0..n).map(|_| vec![0; n]).collect();
    assert!(min_cost_matching(&cost).is_none());
}

#[test]
fn handles_zero_cost_matrix() {
    let cost = vec![vec![0; 4]; 4];
    let (c, assignment) = min_cost_matching(&cost).unwrap();
    assert_eq!(c, 0);
    let mut sorted = assignment.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1, 2, 3]);
}
