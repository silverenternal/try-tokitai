# -*- coding: utf-8 -*-
"""
05_feature_robustness.py — Phase E: 特征子集鲁棒性 (Hypothesis #5)
分析不同特征子集对三个模型性能的影响
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# 设置标准输出编码为utf-8
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8')

import numpy as np
import pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from sklearn.base import clone
from utils import (
    load_data, evaluate_model_cv, create_knn, create_decision_tree,
    create_random_forest, StandardScaler,
    save_figure, save_results_csv, setup_plot_style, RESULTS_DIR
)

print("=" * 60)
print("Phase E: 特征子集鲁棒性分析 (Hypothesis #5)")
print("=" * 60)

# 加载数据
X, y, feature_names, target_names = load_data()
print(f"\n数据加载完成: {X.shape[0]} 样本, {X.shape[1]} 特征")
print(f"特征: {feature_names}")

# 加载最优参数
try:
    best_params_path = os.path.join(RESULTS_DIR, 'hyperparameter_sensitivity', 'best_hyperparameters.csv')
    if os.path.exists(best_params_path):
        df_params = pd.read_csv(best_params_path)
        best_k = int(df_params.iloc[0]['Best_Hyperparameter'].split('=')[1])
        best_depth_str = df_params.iloc[1]['Best_Hyperparameter'].split('=')[1]
        best_depth = None if best_depth_str == 'None' else int(best_depth_str)
        best_n = int(df_params.iloc[2]['Best_Hyperparameter'].split('=')[1])
        print(f"最优参数: k={best_k}, max_depth={best_depth}, n_estimators={best_n}")
    else:
        raise FileNotFoundError
except:
    print("使用默认最优参数...")
    best_k, best_depth, best_n = 5, 3, 50

# ============================================
# E1: 特征子集实验
# ============================================
print("\n" + "-" * 40)
print("E1: 不同特征子集下的模型性能")
print("-" * 40)

# 特征子集定义
feature_subsets = {
    'F1: 全部特征': [0, 1, 2, 3],
    'F2: 仅花瓣': [2, 3],
    'F3: 仅花萼': [0, 1],
    'F4: 花萼长+花瓣长': [0, 2],
    'F5: 花萼宽+花瓣宽': [1, 3]
}

models = {
    'KNN': (create_knn(k=best_k), True),
    '决策树': (create_decision_tree(max_depth=best_depth), False),
    '随机森林': (create_random_forest(n_estimators=best_n), False)
}

N_REPEATS = 5

# 存储结果: {subset_name: {model_name: {'mean': ..., 'std': ..., 'scores': [...]}}}
all_results = {}

for subset_name, subset_idx in feature_subsets.items():
    print(f"\n--- {subset_name} (特征索引: {subset_idx}) ---")
    X_subset = X[:, subset_idx]
    
    all_results[subset_name] = {}
    for model_name, (model, scale_flag) in models.items():
        scores, mean_score, std_score = evaluate_model_cv(
            model, X_subset, y, n_repeats=N_REPEATS, scale=scale_flag
        )
        all_results[subset_name][model_name] = {
            'mean': mean_score,
            'std': std_score,
            'scores': scores
        }
        print(f"  {model_name}: {mean_score:.4f} ± {std_score:.4f}")

# ============================================
# E2: 可视化
# ============================================
print("\n" + "-" * 40)
print("E2: 绘制对比图")
print("-" * 40)

setup_plot_style()

# 图1: 分组柱状图
fig, ax = plt.subplots(figsize=(14, 7))
subset_names = list(feature_subsets.keys())
model_names = ['KNN', '决策树', '随机森林']
colors = {'KNN': '#2E86AB', '决策树': '#A23B72', '随机森林': '#F18F01'}
markers = {'KNN': '///', '决策树': '\\\\\\', '随机森林': 'xxx'}

x = np.arange(len(subset_names))
width = 0.25

for i, model_name in enumerate(model_names):
    means = [all_results[sn][model_name]['mean'] for sn in subset_names]
    stds = [all_results[sn][model_name]['std'] for sn in subset_names]
    bars = ax.bar(x + i * width, means, width, yerr=stds,
                  label=model_name, color=colors[model_name], alpha=0.8,
                  capsize=4, error_kw={'capthick': 2})
    
    # 标注数值
    for bar, mean in zip(bars, means):
        ax.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.01,
                f'{mean:.3f}', ha='center', va='bottom', fontsize=8, rotation=0)

ax.set_xlabel('特征子集')
ax.set_ylabel('10折交叉验证准确率')
ax.set_title('不同特征子集下三个模型的性能对比', fontweight='bold')
ax.set_xticks(x + width)
ax.set_xticklabels(subset_names, rotation=15, ha='right')
ax.legend(loc='lower left')
ax.grid(True, alpha=0.3, axis='y')
ax.set_ylim(0.55, 1.05)
plt.tight_layout()
save_figure(fig, 'feature_subset_comparison.png', 'feature_robustness')

# 图2: 热力图 — 特征子集×模型 的准确率
fig, ax = plt.subplots(figsize=(10, 6))
heatmap_data = np.array([
    [all_results[sn][mn]['mean'] for mn in model_names]
    for sn in subset_names
])

im = ax.imshow(heatmap_data, cmap='YlOrRd', aspect='auto', vmin=0.6, vmax=1.0)

# 标注数值
for i in range(len(subset_names)):
    for j in range(len(model_names)):
        val = heatmap_data[i, j]
        ax.text(j, i, f'{val:.3f}', ha='center', va='center', 
                fontsize=11, fontweight='bold',
                color='white' if val < 0.85 else 'black')

ax.set_xticks(range(len(model_names)))
ax.set_yticks(range(len(subset_names)))
ax.set_xticklabels(model_names)
ax.set_yticklabels(subset_names)
ax.set_title('特征子集 × 模型 — 准确率热力图', fontweight='bold')
plt.colorbar(im, ax=ax, label='准确率')
plt.tight_layout()
save_figure(fig, 'feature_subset_heatmap.png', 'feature_robustness')

# ============================================
# 保存结果
# ============================================
print("\n保存结果...")
records = []
for subset_name in subset_names:
    for model_name in model_names:
        records.append({
            'feature_subset': subset_name,
            'feature_indices': str(feature_subsets[subset_name]),
            'model': model_name,
            'mean_accuracy': round(all_results[subset_name][model_name]['mean'], 4),
            'std_accuracy': round(all_results[subset_name][model_name]['std'], 4)
        })

df_features = pd.DataFrame(records)
save_results_csv(df_features, 'feature_robustness_results.csv', 'feature_robustness')

# ============================================
# 假设#5 检验结论
# ============================================
print("\n" + "-" * 40)
print("假设#5 检验结论")
print("-" * 40)

# 计算特征减少时的性能降幅
full_name = 'F1: 全部特征'
petal_name = 'F2: 仅花瓣'
sepal_name = 'F3: 仅花萼'

print(f"\n从全部特征 → 仅花瓣特征:")
for model_name in model_names:
    full_acc = all_results[full_name][model_name]['mean']
    petal_acc = all_results[petal_name][model_name]['mean']
    drop = full_acc - petal_acc
    print(f"  {model_name}: {full_acc:.4f} → {petal_acc:.4f} (下降 {drop:.4f})")

print(f"\n从全部特征 → 仅花萼特征:")
for model_name in model_names:
    full_acc = all_results[full_name][model_name]['mean']
    sepal_acc = all_results[sepal_name][model_name]['mean']
    drop = full_acc - sepal_acc
    print(f"  {model_name}: {full_acc:.4f} → {sepal_acc:.4f} (下降 {drop:.4f})")

# 判断假设: DT降幅最小, RF降幅最大
petal_drops = {}
sepal_drops = {}
for model_name in model_names:
    petal_drops[model_name] = (all_results[full_name][model_name]['mean'] - 
                                all_results[petal_name][model_name]['mean'])
    sepal_drops[model_name] = (all_results[full_name][model_name]['mean'] - 
                                all_results[sepal_name][model_name]['mean'])

print(f"\n  花瓣特征下降: {petal_drops}")
print(f"  花萼特征下降: {sepal_drops}")

# DT降幅最小
dt_min_petal = petal_drops['决策树'] <= petal_drops['KNN'] and petal_drops['决策树'] <= petal_drops['随机森林']
dt_min_sepal = sepal_drops['决策树'] <= sepal_drops['KNN'] and sepal_drops['决策树'] <= sepal_drops['随机森林']
# RF降幅最大
rf_max_petal = petal_drops['随机森林'] >= petal_drops['KNN'] and petal_drops['随机森林'] >= petal_drops['决策树']
rf_max_sepal = sepal_drops['随机森林'] >= sepal_drops['KNN'] and sepal_drops['随机森林'] >= sepal_drops['决策树']

if dt_min_petal and dt_min_sepal:
    print(f"\n✅ 假设#5 确认(部分): 决策树对特征减少的鲁棒性最强")
else:
    print(f"\n⚠️ 假设#5 部分否证: 决策树并非始终最鲁棒")
    best_model_petal = min(petal_drops, key=petal_drops.get)
    best_model_sepal = min(sepal_drops, key=sepal_drops.get)
    print(f"   花瓣特征下降最少的模型: {best_model_petal}")
    print(f"   花萼特征下降最少的模型: {best_model_sepal}")

if rf_max_petal and rf_max_sepal:
    print(f"✅ 假设#5 确认(部分): 随机森林对特征减少的鲁棒性最弱")
else:
    print(f"⚠️ 假设#5 部分否证: 随机森林并非始终最脆弱")
    worst_model_petal = max(petal_drops, key=petal_drops.get)
    worst_model_sepal = max(sepal_drops, key=sepal_drops.get)
    print(f"   花瓣特征下降最多的模型: {worst_model_petal}")
    print(f"   花萼特征下降最多的模型: {worst_model_sepal}")

if dt_min_petal and dt_min_sepal and rf_max_petal and rf_max_sepal:
    print(f"\n✅ 假设#5 完全确认!")
else:
    print(f"\n⚠️ 假设#5 部分否证")

print("\n" + "=" * 60)
print("Phase E 完成!")
print("=" * 60)
