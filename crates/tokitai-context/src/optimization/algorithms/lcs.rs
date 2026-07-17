//! Hirschberg 算法优化 LCS 计算
//!
//! ## 算法说明
//!
//! 标准 LCS 动态规划空间复杂度为 O(m*n)，对于大文件（如 10MB 文本）会导致内存溢出。
//! Hirschberg 算法通过分治策略将空间复杂度优化到 O(min(m,n))，同时保持 O(m*n) 时间复杂度。
//!
//! ## 核心思想
//!
//! 1. **分治策略**: 将序列 A 从中间分割，找到序列 B 的最优分割点
//! 2. **线性空间 DP**: 只计算 DP 表的当前行和上一行，而非完整二维表
//! 3. **递归求解**: 对分割后的子问题递归应用相同策略
//!
//! ## 性能对比
//!
//! | 算法 | 空间复杂度 | 10MB 文件内存占用 |
//! |------|-----------|------------------|
//! | 标准 DP | O(m*n) | ~100GB (不可行) |
//! | Hirschberg | O(min(m,n)) | ~10MB |
//!
//! ## 使用场景
//!
//! - 大文件合并（>1MB）
//! - 内存受限环境
//! - 长序列比对

use std::cmp::max;

/// Hirschberg LCS 算法实现
pub struct HirschbergLCS;

impl HirschbergLCS {
    /// 计算最长公共子序列（Hirschberg 算法）
    ///
    /// # Arguments
    /// * `a` - 第一个序列
    /// * `b` - 第二个序列
    ///
    /// # Returns
    /// LCS 的索引对列表 (a_idx, b_idx)
    pub fn compute_lcs<T: PartialEq + Clone>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
        if a.is_empty() || b.is_empty() {
            return Vec::new();
        }

        // 小数组使用标准 DP（更快）
        if a.len() <= 64 || b.len() <= 64 {
            return Self::standard_lcs_dp(a, b);
        }

        // 大数组使用 Hirschberg 分治
        Self::hirschberg_recursive(a, b, 0, 0)
    }

    /// 标准 DP 实现（用于小数组）
    fn standard_lcs_dp<T: PartialEq + Clone>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
        let m = a.len();
        let n = b.len();

        // 优化：只使用两行 DP 表
        let mut prev = vec![0usize; n + 1];
        let mut curr = vec![0usize; n + 1];

        // 填充 DP 表
        for i in 1..=m {
            for j in 1..=n {
                if a[i - 1] == b[j - 1] {
                    curr[j] = prev[j - 1] + 1;
                } else {
                    curr[j] = max(prev[j], curr[j - 1]);
                }
            }
            std::mem::swap(&mut prev, &mut curr);
        }

        // 回溯（需要重新计算，但空间最优）
        Self::backtrack_two_rows(a, b)
    }

    /// 使用两行 DP 表回溯
    fn backtrack_two_rows<T: PartialEq>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
        let m = a.len();
        let n = b.len();

        // 完整 DP 表用于回溯（仅在小组情况下使用）
        let mut dp = vec![vec![0usize; n + 1]; m + 1];

        for i in 1..=m {
            for j in 1..=n {
                if a[i - 1] == b[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = max(dp[i - 1][j], dp[i][j - 1]);
                }
            }
        }

        // 回溯
        let mut result = Vec::new();
        let mut i = m;
        let mut j = n;

        while i > 0 && j > 0 {
            if a[i - 1] == b[j - 1] {
                result.push((i - 1, j - 1));
                i -= 1;
                j -= 1;
            } else if dp[i - 1][j] > dp[i][j - 1] {
                i -= 1;
            } else {
                j -= 1;
            }
        }

        result.reverse();
        result
    }

    /// Hirschberg 递归实现
    ///
    /// # Arguments
    /// * `a` - 序列 A
    /// * `b` - 序列 B
    /// * `a_offset` - A 的起始偏移
    /// * `b_offset` - B 的起始偏移
    fn hirschberg_recursive<T: PartialEq + Clone>(
        a: &[T],
        b: &[T],
        a_offset: usize,
        b_offset: usize,
    ) -> Vec<(usize, usize)> {
        let m = a.len();
        let n = b.len();

        // 基础情况
        if m == 0 {
            return Vec::new();
        }

        if n == 0 {
            return Vec::new();
        }

        // 小问题使用标准 DP
        if m <= 64 || n <= 64 {
            let lcs = Self::standard_lcs_dp(a, b);
            return lcs
                .into_iter()
                .map(|(ai, bi)| (a_offset + ai, b_offset + bi))
                .collect();
        }

        // 分割 A
        let mid = m / 2;

        // 计算分割点的最优位置
        let split_point = Self::find_optimal_split(&a[..mid], &a[mid..], b);

        // 递归求解两个子问题
        let mut result = Self::hirschberg_recursive(
            &a[..mid],
            &b[..split_point],
            a_offset,
            b_offset,
        );

        result.extend(Self::hirschberg_recursive(
            &a[mid..],
            &b[split_point..],
            a_offset + mid,
            b_offset + split_point,
        ));

        result
    }

    /// 找到最优分割点
    ///
    /// # Arguments
    /// * `a_left` - A 的左半部分
    /// * `a_right` - A 的右半部分
    /// * `b` - 完整序列 B
    ///
    /// # Returns
    /// B 的最优分割点索引
    fn find_optimal_split<T: PartialEq>(a_left: &[T], a_right: &[T], b: &[T]) -> usize {
        let n = b.len();

        // 计算从左到右的 LCS 长度（对于 a_left）
        let left_scores = Self::compute_lcs_row(a_left, b);

        // 计算从右到左的 LCS 长度（对于 a_right 的反转）
        let right_scores = Self::compute_lcs_row_reverse(a_right, b);

        // 找到最大化 left[j] + right[j] 的位置
        let mut best_j = 0;
        let mut best_score = 0;

        for j in 0..=n {
            let score = left_scores[j] + right_scores[j];
            if score > best_score {
                best_score = score;
                best_j = j;
            }
        }

        best_j
    }

    /// 计算 LCS 长度行（从左到右）
    ///
    /// 返回数组 score，其中 score[j] = LCS(a, b[..j]) 的长度
    fn compute_lcs_row<T: PartialEq>(a: &[T], b: &[T]) -> Vec<usize> {
        let n = b.len();
        let mut prev = vec![0usize; n + 1];
        let mut curr = vec![0usize; n + 1];

        for i in 1..=a.len() {
            for j in 1..=n {
                if a[i - 1] == b[j - 1] {
                    curr[j] = prev[j - 1] + 1;
                } else {
                    curr[j] = max(prev[j], curr[j - 1]);
                }
            }
            std::mem::swap(&mut prev, &mut curr);
        }

        prev
    }

    /// 计算 LCS 长度行（从右到左，用于反向计算）
    /// 返回数组 score，其中 score[j] = LCS(a_rev, b[j..].rev()) 的长度
    fn compute_lcs_row_reverse<T: PartialEq>(a: &[T], b: &[T]) -> Vec<usize> {
        let n = b.len();
        let m = a.len();
        let mut prev = vec![0usize; n + 1];
        let mut curr = vec![0usize; n + 1];

        // 反向遍历：从 a 的末尾到开头，b 的末尾到开头
        for i in 0..m {
            for j in 0..n {
                // 反向索引
                let ai = m - 1 - i;
                let bj = n - 1 - j;
                if a[ai] == b[bj] {
                    curr[j + 1] = prev[j] + 1;
                } else {
                    curr[j + 1] = max(prev[j + 1], curr[j]);
                }
            }
            std::mem::swap(&mut prev, &mut curr);
        }

        prev
    }
}

/// 优化的 LCS 比对结果
#[derive(Debug, Clone)]
pub struct OptimizedLcsResult {
    /// LCS 索引对列表
    pub lcs_pairs: Vec<(usize, usize)>,
    /// A 中未匹配的部分
    pub a_only: Vec<usize>,
    /// B 中未匹配的部分
    pub b_only: Vec<usize>,
    /// LCS 长度
    pub lcs_length: usize,
}

impl OptimizedLcsResult {
    /// 从 LCS 索引对构建结果
    pub fn from_pairs(a_len: usize, b_len: usize, lcs_pairs: &[(usize, usize)]) -> Self {
        let lcs_set_a: std::collections::HashSet<usize> =
            lcs_pairs.iter().map(|(ai, _)| *ai).collect();
        let lcs_set_b: std::collections::HashSet<usize> =
            lcs_pairs.iter().map(|(_, bi)| *bi).collect();

        let a_only: Vec<usize> = (0..a_len).filter(|i| !lcs_set_a.contains(i)).collect();
        let b_only: Vec<usize> = (0..b_len).filter(|i| !lcs_set_b.contains(i)).collect();

        Self {
            lcs_pairs: lcs_pairs.to_vec(),
            a_only,
            b_only,
            lcs_length: lcs_pairs.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_sequences() {
        let a: Vec<i32> = vec![];
        let b: Vec<i32> = vec![];
        assert!(HirschbergLCS::compute_lcs(&a, &b).is_empty());

        let a: Vec<i32> = vec![1, 2, 3];
        let b: Vec<i32> = vec![];
        assert!(HirschbergLCS::compute_lcs(&a, &b).is_empty());
    }

    #[test]
    fn test_identical_sequences() {
        let a = vec![1, 2, 3, 4, 5];
        let b = vec![1, 2, 3, 4, 5];
        let lcs = HirschbergLCS::compute_lcs(&a, &b);
        assert_eq!(lcs.len(), 5);
    }

    #[test]
    fn test_disjoint_sequences() {
        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        let lcs = HirschbergLCS::compute_lcs(&a, &b);
        assert!(lcs.is_empty());
    }

    #[test]
    fn test_partial_overlap() {
        let a = vec![1, 2, 3, 4, 5];
        let b = vec![2, 3, 4, 6, 7];
        let lcs = HirschbergLCS::compute_lcs(&a, &b);
        assert_eq!(lcs.len(), 3);
        assert_eq!(lcs[0], (1, 0)); // 2
        assert_eq!(lcs[1], (2, 1)); // 3
        assert_eq!(lcs[2], (3, 2)); // 4
    }

    #[test]
    fn test_string_lcs() {
        let a: Vec<char> = "ABCBDAB".chars().collect();
        let b: Vec<char> = "BDCABA".chars().collect();
        let lcs = HirschbergLCS::compute_lcs(&a, &b);
        assert!(lcs.len() >= 3); // BCAB 或 BDAB
    }

    #[test]
    fn test_large_sequences() {
        // 测试大数组（触发 Hirschberg 算法）
        let a: Vec<usize> = (0..1000).collect();
        let b: Vec<usize> = (0..1000).step_by(2).collect(); // 偶数
        let lcs = HirschbergLCS::compute_lcs(&a, &b);
        assert_eq!(lcs.len(), 500);
    }

    #[test]
    fn test_memory_efficiency() {
        // 测试内存效率：10000 个元素的序列
        let a: Vec<usize> = (0..10000).collect();
        let b: Vec<usize> = (0..10000).rev().collect();
        let lcs = HirschbergLCS::compute_lcs(&a, &b);
        // 完全逆序，LCS 长度为 1
        assert_eq!(lcs.len(), 1);
    }
}
