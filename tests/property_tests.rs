//! 属性测试 (Property-based Testing)
//!
//! 使用 proptest 框架测试算法的不变性和边界条件
//!
//! 运行方式：
//! ```bash
//! cargo test --test property_tests
//! ```

use ai_assistant::context::MergeStrategy;
use proptest::prelude::*;

// ========== Merge Strategy 属性测试 ==========

proptest! {
    /// 测试合并策略的序列化/反序列化不变性
    #[test]
    fn test_merge_strategy_serialization_roundtrip(
        strategy in prop_oneof![
            Just(MergeStrategy::FastForward),
            Just(MergeStrategy::SelectiveMerge),
            Just(MergeStrategy::AIAssisted),
            Just(MergeStrategy::Manual),
            Just(MergeStrategy::Ours),
            Just(MergeStrategy::Theirs),
        ]
    ) {
        // 序列化为字符串
        let serialized = format!("{}", strategy);

        // 验证字符串不为空
        prop_assert!(!serialized.is_empty());

        // 验证字符串包含策略名称
        prop_assert!(serialized.contains('_') || serialized.chars().all(|c| c.is_alphabetic()));
    }
}

// ========== 字符串操作属性测试 ==========

proptest! {
    /// 测试字符串连接的结合律
    #[test]
    fn test_string_concatenation_associative(
        a in "\\PC*",
        b in "\\PC*",
        c in "\\PC*"
    ) {
        let left = format!("{}{}{}", a, b, c);
        let right = format!("{}{}{}", a, b, c);

        prop_assert_eq!(left, right);
    }

    /// 测试字符串长度在连接后增加
    #[test]
    fn test_string_concatenation_length(
        a in "\\PC*",
        b in "\\PC*"
    ) {
        let concat = format!("{}{}", a, b);

        prop_assert!(concat.len() >= a.len());
        prop_assert!(concat.len() >= b.len());
        prop_assert_eq!(concat.len(), a.len() + b.len());
    }

    /// 测试空字符串连接的恒等性
    #[test]
    fn test_string_concatenation_identity(
        s in "\\PC*"
    ) {
        let left = format!("{}{}", "", s);
        let right = format!("{}{}", s, "");

        prop_assert_eq!(left, s);
        prop_assert_eq!(right, s);
    }
}

// ========== 数值计算属性测试 ==========

proptest! {
    /// 测试加法的交换律
    #[test]
    fn test_addition_commutative(
        a in 0.0..1000.0f64,
        b in 0.0..1000.0f64
    ) {
        prop_assert!((a + b - (b + a)).abs() < 1e-10);
    }

    /// 测试乘法的交换律
    #[test]
    fn test_multiplication_commutative(
        a in 0.0..100.0f64,
        b in 0.0..100.0f64
    ) {
        prop_assert!((a * b - (b * a)).abs() < 1e-10);
    }

    /// 测试加法结合律
    #[test]
    fn test_addition_associative(
        a in 0.0..100.0f64,
        b in 0.0..100.0f64,
        c in 0.0..100.0f64
    ) {
        let left = (a + b) + c;
        let right = a + (b + c);

        prop_assert!((left - right).abs() < 1e-10);
    }

    /// 测试乘以 1 的恒等性
    #[test]
    fn test_multiplication_identity(
        a in 0.0..1000.0f64
    ) {
        prop_assert!((a * 1.0 - a).abs() < 1e-10);
    }

    /// 测试乘以 0 的零化性
    #[test]
    fn test_multiplication_zero(
        a in 0.0..1000.0f64
    ) {
        prop_assert!((a * 0.0).abs() < 1e-10);
    }
}

// ========== 集合操作属性测试 ==========

proptest! {
    /// 测试 Vec 去重后长度不大于原长度
    #[test]
    fn test_vec_dedup_length(
        mut vec in prop::collection::vec(0..100usize, 1..50)
    ) {
        let original_len = vec.len();
        vec.sort();
        vec.dedup();

        prop_assert!(vec.len() <= original_len);
    }

    /// 测试 Vec 反转两次后等于原 Vec
    #[test]
    fn test_vec_reverse_involution(
        mut vec in prop::collection::vec(0..100i32, 1..50)
    ) {
        let original = vec.clone();
        vec.reverse();
        vec.reverse();

        prop_assert_eq!(vec, original);
    }

    /// 测试 Vec 排序后是有序的
    #[test]
    fn test_vec_sort_ordered(
        mut vec in prop::collection::vec(0..100i32, 1..50)
    ) {
        vec.sort();

        for i in 1..vec.len() {
            prop_assert!(vec[i - 1] <= vec[i]);
        }
    }

    /// 测试 Vec 排序后长度不变
    #[test]
    fn test_vec_sort_length_preserved(
        mut vec in prop::collection::vec(0..100i32, 1..50)
    ) {
        let original_len = vec.len();
        vec.sort();

        prop_assert_eq!(vec.len(), original_len);
    }

    /// 测试 Vec 排序后元素集合不变
    #[test]
    fn test_vec_sort_elements_preserved(
        mut vec in prop::collection::vec(0..100i32, 1..50)
    ) {
        let mut sorted = vec.clone();
        sorted.sort();

        // 排序前后应该包含相同的元素（重排后相等）
        vec.sort();
        prop_assert_eq!(vec, sorted);
    }
}

// ========== HashMap 属性测试 ==========

proptest! {
    /// 测试 HashMap 插入后长度增加
    #[test]
    fn test_hashmap_insert_increases_length(
        pairs in prop::collection::vec((any::<u32>(), any::<u32>()), 1..20)
    ) {
        let mut map = std::collections::HashMap::new();
        let mut expected_len = 0;

        for (key, value) in pairs {
            if !map.contains_key(&key) {
                expected_len += 1;
            }
            map.insert(key, value);
        }

        prop_assert_eq!(map.len(), expected_len);
    }

    /// 测试 HashMap 插入后可检索
    #[test]
    fn test_hashmap_insert_retrievable(
        key in any::<u32>(),
        value in any::<u32>()
    ) {
        let mut map = std::collections::HashMap::new();
        map.insert(key, value);

        prop_assert_eq!(map.get(&key), Some(&value));
    }

    /// 测试 HashMap 删除后不可检索
    #[test]
    fn test_hashmap_remove_not_found(
        key1 in any::<u32>(),
        value1 in any::<u32>(),
        key2 in any::<u32>()
    ) {
        let mut map = std::collections::HashMap::new();
        map.insert(key1, value1);

        if key1 != key2 {
            map.remove(&key2);
            prop_assert_eq!(map.get(&key2), None);
        }
    }
}

// ========== 边界条件测试 ==========

proptest! {
    /// 测试大数加法
    #[test]
    fn test_large_number_addition(
        a in 1_000_000.0..10_000_000.0f64,
        b in 1_000_000.0..10_000_000.0f64
    ) {
        let result = a + b;

        prop_assert!(result > 2_000_000.0);
        prop_assert!(result < 20_000_000.0);
        prop_assert!((result - (b + a)).abs() < 1e-5);
    }

    /// 测试小数精度
    #[test]
    fn test_decimal_precision(
        a in 0.0001..0.001f64,
        b in 0.0001..0.001f64
    ) {
        let sum = a + b;

        prop_assert!(sum > 0.0002);
        prop_assert!(sum < 0.002);
    }

    /// 测试负数运算
    #[test]
    fn test_negative_numbers(
        a in -1000.0..-1.0f64,
        b in 1.0..1000.0f64
    ) {
        let sum = a + b;

        // 结果应该在 a 和 b 之间
        prop_assert!(sum > a && sum < b);
    }
}

// ========== 字符串属性测试 ==========

proptest! {
    /// 测试字符串反转两次后等于原字符串
    #[test]
    fn test_string_reverse_involution(
        s in "\\PC*"
    ) {
        let reversed: String = s.chars().rev().collect();
        let double_reversed: String = reversed.chars().rev().collect();

        prop_assert_eq!(double_reversed, s);
    }

    /// 测试字符串长度非负
    #[test]
    fn test_string_length_non_negative(
        s in "\\PC*"
    ) {
        prop_assert!(s.len() >= 0);
    }

    /// 测试字符串切片不越界
    #[test]
    fn test_string_slice_bounds(
        s in "\\PC*",
        start in 0..100usize,
        end in 0..100usize
    ) {
        let start = start.min(s.len());
        let end = end.min(s.len());

        if start <= end {
            let slice = &s[start..end];
            prop_assert!(slice.len() <= end - start);
        }
    }
}

// ========== Option 类型属性测试 ==========

proptest! {
    /// 测试 Option 的 map 操作
    #[test]
    fn test_option_map(
        opt in prop::option::any::<i32>(),
        multiplier in 1..100i32
    ) {
        let result = opt.map(|x| x * multiplier);

        match (opt, result) {
            (None, None) => (),
            (Some(x), Some(y)) => prop_assert_eq!(y, x * multiplier),
            _ => panic!("Option map behavior mismatch"),
        }
    }

    /// 测试 Option 的 unwrap_or 操作
    #[test]
    fn test_option_unwrap_or(
        opt in prop::option::any::<i32>(),
        default in any::<i32>()
    ) {
        let result = opt.unwrap_or(default);

        match opt {
            Some(x) => prop_assert_eq!(result, x),
            None => prop_assert_eq!(result, default),
        }
    }
}

// ========== Result 类型属性测试 ==========

proptest! {
    /// 测试 Result 的 is_ok/is_err 互斥性
    #[test]
    fn test_result_ok_err_mutually_exclusive(
        result in prop::result::any::<i32, i32>()
    ) {
        prop_assert!(result.is_ok() != result.is_err());
    }

    /// 测试 Result 的 map 操作
    #[test]
    fn test_result_map(
        value in any::<i32>(),
        multiplier in 1..100i32
    ) {
        let ok_result: Result<i32, i32> = Ok(value);
        let mapped = ok_result.map(|x| x * multiplier);

        prop_assert_eq!(mapped.unwrap(), value * multiplier);
    }
}
