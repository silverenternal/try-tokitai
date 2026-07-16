# -*- coding: utf-8 -*-
"""
04_learning_curves.py — Phase D: 训练集规模敏感性 (Hypothesis #4)
分析训练集规模对三个模型性能的影响
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
from sklearn.model_selection import StratifiedShuffleSplit
from utils import (
    load_data, create_knn, create_decision_tree, create_random_forest,
    StandardScaler, accuracy_score,
    save_figure, save_results_csv, setup_plot_style, RESULTS_DIR
)

print("=" * 60)
print("Phase D: 训练集规模敏感性分析 (Hypothesis #4)")
print("=" * 60)

# 加载数据
X, y, feature_names, target_names = load_data()
print(f"\n数据加载完成: {X.shape[0]} 样本")

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
# D1: 学习曲线实验
# ============================================
print("\n" + "-" * 40)
print("D1: 不同训练集规模下的性能")
print("-" * 40)

# 训练集比例
train_ratios = [0.10, 0.20, 0.30, 0.40, 0.50, 0.70, 0.85]
train_sizes = [int(X.shape[0] * r) for r in train_ratios]
N_REPEATS = 10

models = {
    'KNN': (create_knn(k=best_k), True),
    '决策树': (create_decision_tree(max_depth=best_depth), False),
    '随机森林': (create_random_forest(n_estimators=best_n), False)
}

# 存储结果: {model_name: {train_ratio: [acc1, acc2, ...]}}
all_results = {name: {r: [] for r in train_ratios} for name in models.keys()}

for ratio in train_ratios:
    n_train = int(X.shape[0] * ratio)
    n_test = X.shape[0] - n_train
    print(f"\n训练集比例: {ratio*100:.0f}% ({n_train} 样本, 测试: {n_test} 样本)")
    
    for repeat in range(N_REPEATS):
        # 使用test_size确保测试集足够大
        test_ratio = 1.0 - ratio
        sss = StratifiedShuffleSplit(n_splits=1, test_size=test_ratio, 
                                     random_state=42 + repeat)
        
        for train_idx, test_idx in sss.split(X, y):
            X_train, X_test = X[train_idx], X[test_idx]
            y_train, y_test = y[train_idx], y[test_idx]
            
            for name, (model, scale_flag) in models.items():
                if scale_flag:
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
                
                all_results[name][ratio].append(accuracy_score(y_test, y_pred))

# 计算均值和标准差
summary = {name: {'means': [], 'stds': []} for name in models.keys()}
for name in models.keys():
    for ratio in train_ratios:
        scores = all_results[name][ratio]
        summary[name]['means'].append(np.mean(scores))
        summary[name]['stds'].append(np.std(scores))

# 打印结果
print("\n" + "=" * 70)
print(f"{'训练比例':<10} {'样本数':<8} {'KNN':<18} {'决策树':<18} {'随机森林':<18}")
print("=" * 70)
for i, ratio in enumerate(train_ratios):
    n = train_sizes[i]
    knn_str = f"{summary['KNN']['means'][i]:.4f} ± {summary['KNN']['stds'][i]:.4f}"
    dt_str = f"{summary['决策树']['means'][i]:.4f} ± {summary['决策树']['stds'][i]:.4f}"
    rf_str = f"{summary['随机森林']['means'][i]:.4f} ± {summary['随机森林']['stds'][i]:.4f}"
    print(f"{ratio*100:<8.0f}% {n:<8} {knn_str:<18} {dt_str:<18} {rf_str:<18}")

# ============================================
# D2: 可视化
# ============================================
print("\n" + "-" * 40)
print("D2: 绘制学习曲线")
print("-" * 40)

setup_plot_style()

fig, ax = plt.subplots(figsize=(12, 7))

colors = {'KNN': '#2E86AB', '决策树': '#A23B72', '随机森林': '#F18F01'}
markers = {'KNN': 'o', '决策树': 's', '随机森林': '^'}

for name in ['KNN', '决策树', '随机森林']:
    means = summary[name]['means']
    stds = summary[name]['stds']
    ax.errorbar(train_sizes, means, yerr=stds, fmt=f'-{markers[name]}', 
                color=colors[name], capsize=5, capthick=2, linewidth=2,
                markersize=8, markerfacecolor=colors[name], 
                markeredgecolor='white', markeredgewidth=2,
                label=name, alpha=0.85)

# 标注小样本阈值
for name in ['KNN', '决策树', '随机森林']:
    means = summary[name]['means']
    # 找到准确率开始显著下降的点（相对于最大准确率下降>2%）
    max_acc = max(means)
    for i, mean in enumerate(means):
        if max_acc - mean > 0.02:
            ax.annotate(f'{name}阈值:{train_sizes[i]}样本',
                       xy=(train_sizes[i], mean),
                       xytext=(train_sizes[i] + 5, mean - 0.03),
                       fontsize=9, color=colors[name],
                       arrowprops=dict(arrowstyle='->', color=colors[name], lw=1))
            break

ax.set_xlabel('训练集样本数')
ax.set_ylabel('测试集准确率')
ax.set_title('学习曲线: 训练集规模对模型性能的影响', fontweight='bold')
ax.legend(loc='lower right')
ax.grid(True, alpha=0.3)
ax.set_xlim(0, X.shape[0] + 10)
ax.set_ylim(0.6, 1.02)

# 添加参考线
ax.axhline(y=0.95, color='gray', linestyle='--', alpha=0.4, label='95% 基准线')

plt.tight_layout()
save_figure(fig, 'learning_curves_comparison.png', 'learning_curves')

# ============================================
# 保存结果
# ============================================
print("\n保存结果...")
records = []
for i, ratio in enumerate(train_ratios):
    for name in ['KNN', '决策树', '随机森林']:
        records.append({
            'train_ratio': ratio,
            'train_samples': train_sizes[i],
            'model': name,
            'mean_accuracy': round(summary[name]['means'][i], 4),
            'std_accuracy': round(summary[name]['stds'][i], 4)
        })

df_learning = pd.DataFrame(records)
save_results_csv(df_learning, 'learning_curve_results.csv', 'learning_curves')

# ============================================
# 假设#4 检验结论
# ============================================
print("\n" + "-" * 40)
print("假设#4 检验结论")
print("-" * 40)

# 计算下降幅度（从最大训练集到最小训练集）
for name in ['KNN', '决策树', '随机森林']:
    means = summary[name]['means']
    full_acc = means[-1]  # 100%训练集
    small_acc = means[1]  # 20%训练集 (30样本)
    drop = full_acc - small_acc
    print(f"\n  {name}:")
    print(f"    全量训练准确率: {full_acc:.4f}")
    print(f"    20%训练准确率: {small_acc:.4f}")
    print(f"    下降幅度: {drop:.4f} ({drop*100:.2f}%)")

# 判断假设
knn_drop = summary['KNN']['means'][-1] - summary['KNN']['means'][1]
dt_drop = summary['决策树']['means'][-1] - summary['决策树']['means'][1]
rf_drop = summary['随机森林']['means'][-1] - summary['随机森林']['means'][1]

print(f"\n  KNN下降: {knn_drop:.4f}")
print(f"  决策树下降: {dt_drop:.4f}")
print(f"  随机森林下降: {rf_drop:.4f}")

if knn_drop >= dt_drop and knn_drop >= rf_drop:
    print(f"\n[确认] 假设#4 确认: KNN性能衰减最快 ({knn_drop:.4f})")
elif rf_drop <= dt_drop and rf_drop <= knn_drop:
    print(f"\n[确认] 假设#4 确认: 随机森林最鲁棒 ({rf_drop:.4f})")
else:
    print(f"\n[警告] 假设#4 部分否证")
    best_robust = min([('KNN', knn_drop), ('决策树', dt_drop), ('随机森林', rf_drop)], key=lambda x: x[1])
    print(f"   最鲁棒模型: {best_robust[0]} (下降{best_robust[1]:.4f})")

print("\n" + "=" * 60)
print("Phase D 完成!")
print("=" * 60)
