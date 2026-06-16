"""
02_performance_equivalence.py — Phase B: 模型性能等价性 (Hypothesis #2)
检验最优参数下三个模型的性能是否存在统计显著差异
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# 设置标准输出编码为utf-8
if sys.stdout.encoding != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8')
    
import numpy as np
import pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import seaborn as sns
from scipy import stats
from sklearn.base import clone
from utils import (
    load_data, get_stratified_kfold, create_knn, create_decision_tree,
    create_random_forest, StandardScaler, accuracy_score,
    save_figure, save_results_csv, setup_plot_style, RESULTS_DIR
)

print("=" * 60)
print("Phase B: 模型性能等价性检验 (Hypothesis #2)")
print("=" * 60)

# 加载数据
X, y, feature_names, target_names = load_data()
print(f"\n数据加载完成: {X.shape[0]} 样本")

# 从Phase A结果中读取最优参数
# 如果CSV不存在，使用默认值
try:
    best_params_path = os.path.join(RESULTS_DIR, 'hyperparameter_sensitivity', 'best_hyperparameters.csv')
    if os.path.exists(best_params_path):
        df_params = pd.read_csv(best_params_path)
        best_k = int(df_params.iloc[0]['Best_Hyperparameter'].split('=')[1])
        best_depth_str = df_params.iloc[1]['Best_Hyperparameter'].split('=')[1]
        best_depth = None if best_depth_str == 'None' else int(best_depth_str)
        best_n = int(df_params.iloc[2]['Best_Hyperparameter'].split('=')[1])
        print(f"\n从Phase A加载最优参数: k={best_k}, max_depth={best_depth}, n_estimators={best_n}")
    else:
        raise FileNotFoundError
except:
    print("\n未找到Phase A结果，使用默认最优参数...")
    best_k = 5
    best_depth = 3
    best_n = 50

# ============================================
# B1: 最优配置下的对比实验
# ============================================
print("\n" + "-" * 40)
print("B1: 最优配置下的重复交叉验证")
print("-" * 40)

N_REPEATS = 10
models = {
    'KNN': create_knn(k=best_k),
    '决策树': create_decision_tree(max_depth=best_depth),
    '随机森林': create_random_forest(n_estimators=best_n)
}
scale_flags = {'KNN': True, '决策树': False, '随机森林': False}

results = {name: [] for name in models.keys()}

for repeat in range(N_REPEATS):
    skf = get_stratified_kfold(random_state=42 + repeat)
    
    for train_idx, test_idx in skf.split(X, y):
        X_train, X_test = X[train_idx], X[test_idx]
        y_train, y_test = y[train_idx], y[test_idx]
        
        for name, model in models.items():
            if scale_flags[name]:
                scaler = StandardScaler()
                X_train_scaled = scaler.fit_transform(X_train)
                X_test_scaled = scaler.transform(X_test)
                clf = clone(model)
                clf.fit(X_train_scaled, y_train)
                y_pred = clf.predict(X_test_scaled)
            else:
                clf = clone(model)
                clf.fit(X_train, y_train)
                y_pred = clf.predict(X_test)
            
            results[name].append(accuracy_score(y_test, y_pred))

# 转换为numpy数组
for name in results:
    results[name] = np.array(results[name])

# 打印统计摘要
print(f"\n{'模型':<10} {'平均准确率':<12} {'标准差':<10} {'最小值':<10} {'最大值':<10}")
print("-" * 50)
for name in ['KNN', '决策树', '随机森林']:
    scores = results[name]
    print(f"{name:<10} {np.mean(scores):.4f}      {np.std(scores):.4f}    {np.min(scores):.4f}    {np.max(scores):.4f}")

# ============================================
# B2: 统计检验
# ============================================
print("\n" + "-" * 40)
print("B2: 统计显著性检验")
print("-" * 40)

model_names = ['KNN', '决策树', '随机森林']
pairs = [('KNN', '决策树'), ('KNN', '随机森林'), ('决策树', '随机森林')]

# 1. 正态性检验 (Shapiro-Wilk)
print("\n1) 正态性检验 (Shapiro-Wilk):")
for name in model_names:
    stat, p_value = stats.shapiro(results[name])
    normal = "正态分布" if p_value > 0.05 else "非正态分布"
    print(f"   {name:<8}: W={stat:.4f}, p={p_value:.4f} -> {normal}")

# 2. 重复测量方差分析 (如果正态)
print("\n2) 重复测量方差分析 (Repeated Measures ANOVA):")
# 使用Friedman检验作为非参数替代
friedman_stat, friedman_p = stats.friedmanchisquare(
    results['KNN'], results['决策树'], results['随机森林']
)
print(f"   Friedman检验: χ²={friedman_stat:.4f}, p={friedman_p:.6f}")
if friedman_p < 0.05:
    print(f"   → 三模型间存在显著差异 (p<0.05)")
else:
    print(f"   → 三模型间无显著差异 (p>=0.05)")

# 3. 配对t检验 (两两比较)
print("\n3) 两两配对t检验 (Bonferroni校正, α'=0.0167):")
alpha_corrected = 0.05 / 3  # Bonferroni校正

t_test_results = []
for name1, name2 in pairs:
    t_stat, p_value = stats.ttest_rel(results[name1], results[name2])
    significant = "显著" if p_value < alpha_corrected else "不显著"
    
    # Cohen's d效应量
    diff = results[name1] - results[name2]
    cohens_d = np.mean(diff) / np.std(diff) if np.std(diff) > 0 else 0
    
    print(f"   {name1} vs {name2}: t={t_stat:.4f}, p={p_value:.6f}, "
          f"d={cohens_d:.4f} -> {significant} (α'={alpha_corrected:.4f})")
    
    t_test_results.append({
        '对比': f'{name1} vs {name2}',
        't统计量': round(t_stat, 4),
        'p值': round(p_value, 6),
        'Bonferroni_alpha': round(alpha_corrected, 4),
        '是否显著': significant,
        "Cohen's d": round(cohens_d, 4)
    })

# 4. Wilcoxon符号秩检验 (非参数补充)
print("\n4) Wilcoxon符号秩检验 (非参数补充):")
for name1, name2 in pairs:
    w_stat, p_value = stats.wilcoxon(results[name1], results[name2])
    significant = "显著" if p_value < alpha_corrected else "不显著"
    print(f"   {name1} vs {name2}: W={w_stat:.1f}, p={p_value:.6f} -> {significant}")

# 保存统计结果
df_stats = pd.DataFrame(t_test_results)
save_results_csv(df_stats, 'statistical_test_results.csv', 'performance_equivalence')

# ============================================
# B3: 可视化
# ============================================
print("\n" + "-" * 40)
print("B3: 可视化")
print("-" * 40)

setup_plot_style()

# 图1: 箱线图
fig, ax = plt.subplots(figsize=(10, 6))
data_to_plot = [results[name] for name in model_names]
bp = ax.boxplot(data_to_plot, labels=model_names, patch_artist=True, 
                widths=0.5, showmeans=True)

colors = ['#2E86AB', '#A23B72', '#F18F01']
for patch, color in zip(bp['boxes'], colors):
    patch.set_facecolor(color)
    patch.set_alpha(0.7)

# 叠加散点图
for i, (name, color) in enumerate(zip(model_names, colors)):
    x = np.random.normal(i + 1, 0.04, size=len(results[name]))
    ax.scatter(x, results[name], alpha=0.4, color=color, s=20)

ax.set_ylabel('10折交叉验证准确率')
ax.set_title('三个模型在最优超参数下的准确率分布对比', fontweight='bold')
ax.grid(True, alpha=0.3, axis='y')
ax.set_ylim(0.8, 1.02)
save_figure(fig, 'accuracy_boxplot.png', 'performance_equivalence')

# 图2: 配对散点图
fig, axes = plt.subplots(1, 3, figsize=(15, 5))
pair_colors = [('#2E86AB', '#A23B72'), ('#2E86AB', '#F18F01'), ('#A23B72', '#F18F01')]

for idx, ((name1, name2), (c1, c2)) in enumerate(zip(pairs, pair_colors)):
    ax = axes[idx]
    ax.scatter(results[name1], results[name2], alpha=0.6, c='gray', edgecolors='black', s=30)
    
    # 对角线 (y=x)
    min_val = min(min(results[name1]), min(results[name2]))
    max_val = max(max(results[name1]), max(results[name2]))
    ax.plot([min_val, max_val], [min_val, max_val], 'r--', alpha=0.5, label='y=x')
    
    ax.set_xlabel(f'{name1} 准确率')
    ax.set_ylabel(f'{name2} 准确率')
    ax.set_title(f'{name1} vs {name2}', fontweight='bold')
    ax.grid(True, alpha=0.3)
    ax.legend()

plt.suptitle('模型准确率配对散点图', fontsize=16, fontweight='bold', y=1.02)
plt.tight_layout()
save_figure(fig, 'paired_scatter.png', 'performance_equivalence')

# ============================================
# 假设检验结论
# ============================================
print("\n" + "-" * 40)
print("假设#2 检验结论")
print("-" * 40)

# 判断假设是否成立
all_not_significant = all(
    stats.ttest_rel(results[n1], results[n2])[1] >= alpha_corrected 
    for n1, n2 in pairs
)

if all_not_significant and friedman_p >= 0.05:
    print("\n✅ 假设#2 确认: 三模型在最优参数下无统计显著差异")
    print("   结果支持\"性能等价性假设\"")
else:
    print("\n⚠️ 假设#2 部分否证: 至少有一对模型存在显著差异")
    for name1, name2 in pairs:
        t, p = stats.ttest_rel(results[name1], results[name2])
        if p < alpha_corrected:
            diff_mean = np.mean(results[name1] - results[name2])
            print(f"   {name1} vs {name2}: 差异 = {diff_mean:.4f} (p={p:.6f})")

print("\n" + "=" * 60)
print("Phase B 完成!")
print("=" * 60)
