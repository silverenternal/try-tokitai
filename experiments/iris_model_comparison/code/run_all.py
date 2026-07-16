"""
run_all.py — 运行所有5个实验Phase
按依赖顺序依次执行:
  Phase A (超参数敏感性) → Phase B (性能等价性) → Phase C (类别级分析)
  → Phase D (学习曲线) → Phase E (特征鲁棒性)
"""

import sys
import os
import time

# 切换到代码目录
script_dir = os.path.dirname(os.path.abspath(__file__))
os.chdir(script_dir)

print("=" * 60)
print("机器学习模型对比 — 全实验运行")
print("Iris数据集: KNN vs 决策树 vs 随机森林")
print("=" * 60)
print(f"\n代码目录: {script_dir}")
print(f"结果目录: {os.path.join(os.path.dirname(script_dir), 'results')}")
print()

phases = [
    ('Phase A: 超参数敏感性', '01_hyperparameter_sensitivity.py'),
    ('Phase B: 性能等价性', '02_performance_equivalence.py'),
    ('Phase C: 类别级分析', '03_class_level_analysis.py'),
    ('Phase D: 学习曲线', '04_learning_curves.py'),
    ('Phase E: 特征鲁棒性', '05_feature_robustness.py'),
]

total_start = time.time()

for phase_name, script_name in phases:
    print(f"\n{'#' * 60}")
    print(f"# 开始: {phase_name}")
    print(f"{'#' * 60}")
    
    start_time = time.time()
    
    # 执行脚本
    script_path = os.path.join(script_dir, script_name)
    exit_code = os.system(f'python "{script_path}"')
    
    elapsed = time.time() - start_time
    
    if exit_code == 0:
        print(f"\n✅ {phase_name} 完成! (耗时: {elapsed:.1f}秒)")
    else:
        print(f"\n❌ {phase_name} 失败! (退出码: {exit_code})")
        print("终止后续实验运行")
        sys.exit(1)

total_elapsed = time.time() - total_start
print(f"\n{'=' * 60}")
print(f"🎉 全部实验完成! 总耗时: {total_elapsed:.1f}秒 ({total_elapsed/60:.1f}分钟)")
print(f"{'=' * 60}")

print(f"\n结果保存在: {os.path.join(os.path.dirname(script_dir), 'results')}")
print("\n生成的结果文件:")
results_dir = os.path.join(os.path.dirname(script_dir), 'results')
for root, dirs, files in os.walk(results_dir):
    level = root.replace(results_dir, '').count(os.sep)
    indent = ' ' * 2 * level
    print(f"{indent}{os.path.basename(root)}/")
    subindent = ' ' * 2 * (level + 1)
    for file in files:
        print(f"{subindent}{file}")
