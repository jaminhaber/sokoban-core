//! Minimum-cost bipartite matching utilities.
//!
//! Sokoban solvers often use a *minimum-cost perfect matching* between boxes and goals
//! as an admissible heuristic. This module provides a small, reliable implementation
//! based on subset dynamic programming, which is typically faster and easier to verify
//! than a handwritten Hungarian algorithm for Sokoban-sized instances.

/// Computes a minimum-cost perfect matching for a square `n × n` cost matrix
/// stored row-major in a flat slice.
///
/// `cost[i * n + j]` is the cost of assigning box `i` to goal `j`. Costs of
/// `i32::MAX` are treated as "impossible edges". The flat representation
/// avoids the `n + 1` allocations of a `Vec<Vec<i32>>` per call — meaningful
/// because the heuristic builds and discards a fresh cost matrix on every
/// cache miss.
///
/// Returns `None` when no perfect matching exists.
///
/// # Complexity
///
/// `O(n * 2^n)` time and `O(2^n)` memory, where `n` is the number of
/// boxes/goals. For Sokoban levels (usually `n <= 20`) this is competitive
/// and very robust.
pub fn min_cost_matching(cost: &[i32], n: usize) -> Option<(i32, Vec<usize>)> {
    if n == 0 {
        return Some((0, Vec::new()));
    }
    if cost.len() != n * n {
        return None;
    }
    if n > 22 {
        // Guardrail: subset DP scales exponentially. Swap to min-cost flow / Hungarian
        // if you intend to support levels with many boxes.
        return None;
    }

    let full = 1usize << n;
    let mut dp = vec![i64::MAX; full];
    let mut parent: Vec<Option<(usize, usize)>> = vec![None; full]; // (prev_mask, chosen_goal)

    dp[0] = 0;

    for mask in 0..full {
        let i = mask.count_ones() as usize;
        if i >= n {
            continue;
        }
        let base = dp[mask];
        if base == i64::MAX {
            continue;
        }

        for g in 0..n {
            if (mask & (1 << g)) != 0 {
                continue;
            }
            let c = cost[i * n + g];
            if c == i32::MAX {
                continue;
            }
            let nm = mask | (1 << g);
            let cand = base + c as i64;
            if cand < dp[nm] {
                dp[nm] = cand;
                parent[nm] = Some((mask, g));
            }
        }
    }

    let best = dp[full - 1];
    if best == i64::MAX || best > i32::MAX as i64 {
        return None;
    }

    // Reconstruct assignment[box_i] = goal_j
    let mut assignment = vec![0usize; n];
    let mut mask = full - 1;
    for box_i in (0..n).rev() {
        let (pm, g) = parent[mask]?;
        assignment[box_i] = g;
        mask = pm;
    }

    Some((best as i32, assignment))
}
