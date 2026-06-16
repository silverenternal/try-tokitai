"""
01_hyperparameter_sensitivity.py — Phase A: 超参数敏感性差异 (Hypothesis #1)
分析 KNN、决策树、随机森林对各自核心超参数的敏感性
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import numpy as np
import pandas as pd
from utils import (
    load_data, evaluate_model_cv, create_knn, create_decision_tree,
    create_random_forest, save_figure, save_results_csv,
    plot_hyperparameter_sensitivity, RESULTS_DIR
)
import time

print("=" * 60)
print("Phase A: 超参数敏感性分析 (Hypothesis #1)")
print("=" * 60)

# 加载数据
X, y, feature_names, target_names = load_data()
print(f"\n数据加载完成: {X.shape[0]} 样本, {X.shape[1]} 特征, {len(np.unique(y))} 类别")

# ============================================
# A1: KNN — k值敏感性
# ============================================
print("\n" + "-" * 40)
print("A1: KNN — k值敏感性分析")
print("-" * 40)

k_values = [1, 3, 5, 7, 9, 11, 15, 21]
knn_results = {'k': [], 'mean_acc': [], 'std_acc': [], 'all_scores': []}

for k in k_values:
    model = create_knn(k=k)
    scores, mean_score, std_score = evaluate_model_cv(model, X, y, scale=True)
    knn_results['k'].append(k)
    knn_results['mean_acc'].append(mean_score)
    knn_results['std_acc'].append(std_score)
    knn_results['all_scores'].append(scores)
    print(f"  k={k:2d}: 准确率 = {mean_score:.4f} ± {std_score:.4f}")

# 找到最优k值
best_k_idx = np.argmax(knn_results['mean_acc'])
best_k = k_values[best_k_idx]
best_k_acc = knn_results['mean_acc'][best_k_idx]
print(f"\n  [最优] k = {best_k}, 准确率 = {best_k_acc:.4f}")

# 绘制KNN敏感性曲线
fig = plot_hyperparameter_sensitivity(
    k_values, knn_results['mean_acc'], knn_results['std_acc'],
    param_name='k (邻居数量)', model_name='KNN', color='#2E86AB'
)
save_figure(fig, 'knn_k_sensitivity.png', 'hyperparameter_sensitivity')

# ============================================
# A2: 决策树 — max_depth敏感性
# ============================================
print("\n" + "-" * 40)
print("A2: 决策树 — max_depth敏感性分析")
print("-" * 40)

depth_values = [1, 2, 3, 4, 5, 6, 8, 10, None]
dt_results = {'max_depth': [], 'mean_acc': [], 'std_acc': [], 'all_scores': []}

for depth in depth_values:
    model = create_decision_tree(max_depth=depth)
    scores, mean_score, std_score = evaluate_model_cv(model, X, y, scale=False)
    dt_results['max_depth'].append(depth)
    dt_results['mean_acc'].append(mean_score)
    dt_results['std_acc'].append(std_score)
    dt_results['all_scores'].append(scores)
    depth_str = str(depth) if depth is not None else 'None'
    print(f"  max_depth={depth_str:4s}: 准确率 = {mean_score:.4f} ± {std_score:.4f}")

# 找到最优depth值
best_dt_idx = np.argmax(dt_results['mean_acc'])
best_depth = depth_values[best_dt_idx]
best_dt_acc = dt_results['mean_acc'][best_dt_idx]
print(f"\n  [最优] max_depth = {best_depth}, 准确率 = {best_dt_acc:.4f}")

# 绘制决策树敏感性曲线
fig = plot_hyperparameter_sensitivity(
    depth_values, dt_results['mean_acc'], dt_results['std_acc'],
    param_name='max_depth (最大深度)', model_name='决策树', color='#A23B72'
)
save_figure(fig, 'dt_depth_sensitivity.png', 'hyperparameter_sensitivity')

# ============================================
# A3: 随机森林 — n_estimators敏感性
# ============================================
print("\n" + "-" * 40)
print("A3: 随机森林 — n_estimators敏感性分析")
print("-" * 40)

n_values = [1, 5, 10, 20, 50, 100, 200, 500]
rf_results = {'n_estimators': [], 'mean_acc': [], 'std_acc': [], 'all_scores': []}

for n in n_values:
    model = create_random_forest(n_estimators=n)
    scores, mean_score, std_score = evaluate_model_cv(model, X, y, scale=False)
    rf_results['n_estimators'].append(n)
    rf_results['mean_acc'].append(mean_score)
    rf_results['std_acc'].append(std_score)
    rf_results['all_scores'].append(scores)
    print(f"  n_estimators={n:3d}: 准确率 = {mean_score:.4f} ± {std_score:.4f}")

# 找到最优n值
best_rf_idx = np.argmax(rf_results['mean_acc'])
best_n = n_values[best_rf_idx]
best_rf_acc = rf_results['mean_acc'][best_rf_idx]
print(f"\n  [最优] n_estimators = {best_n}, 准确率 = {best_rf_acc:.4f}")

# 绘制随机森林敏感性曲线
fig = plot_hyperparameter_sensitivity(
    n_values, rf_results['mean_acc'], rf_results['std_acc'],
    param_name='n_estimators (树数量)', model_name='随机森林', color='#F18F01'
)
save_figure(fig, 'rf_estimators_sensitivity.png', 'hyperparameter_sensitivity')

# ============================================
# 汇总结果保存
# ============================================
print("\n" + "-" * 40)
print("保存汇总结果")
print("-" * 40)

# 保存CSV
df_knn = pd.DataFrame({
    'k': knn_results['k'],
    'mean_accuracy': knn_results['mean_acc'],
    'std_accuracy': knn_results['std_acc']
})
df_dt = pd.DataFrame({
    'max_depth': [str(d) for d in dt_results['max_depth']],
    'mean_accuracy': dt_results['mean_acc'],
    'std_accuracy': dt_results['std_acc']
})
df_rf = pd.DataFrame({
    'n_estimators': rf_results['n_estimators'],
    'mean_accuracy': rf_results['mean_acc'],
    'std_accuracy': rf_results['std_acc']
})

save_results_csv(df_knn, 'knn_sensitivity.csv', 'hyperparameter_sensitivity')
save_results_csv(df_dt, 'dt_sensitivity.csv', 'hyperparameter_sensitivity')
save_results_csv(df_rf, 'rf_sensitivity.csv', 'hyperparameter_sensitivity')

# 最优参数汇总
best_params = pd.DataFrame({
    'Model': ['KNN', '决策树', '随机森林'],
    'Best_Hyperparameter': [f'k={best_k}', f'max_depth={best_depth}', f'n_estimators={best_n}'],
    'Best_Accuracy': [f'{best_k_acc:.4f}', f'{best_dt_acc:.4f}', f'{best_rf_acc:.4f}']
})
save_results_csv(best_params, 'best_hyperparameters.csv', 'hyperparameter_sensitivity')

print("\n" + "=" * 60)
print("Phase A 完成!")
print(f"  最优 KNN:       k = {best_k}, 准确率 = {best_k_acc:.4f}")
print(f"  最优 决策树:    max_depth = {best_depth}, 准确率 = {best_dt_acc:.4f}")
print(f"  最优 随机森林:  n_estimators = {best_n}, 准确率 = {best_rf_acc:.4f}")
print("=" * 60)
