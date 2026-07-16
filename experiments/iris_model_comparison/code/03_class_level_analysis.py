# -*- coding: utf-8 -*-
"""
03_class_level_analysis.py — Phase C: 类别级性能差异 (Hypothesis #3)
分析每个模型在三个类别上的性能差异
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
import seaborn as sns
from sklearn.base import clone
from sklearn.metrics import confusion_matrix, classification_report
from scipy.stats import chi2
from utils import (
    load_data, get_stratified_kfold, create_knn, create_decision_tree,
    create_random_forest, StandardScaler, accuracy_score,
    precision_score, recall_score, f1_score,
    save_figure, save_results_csv, setup_plot_style, RESULTS_DIR
)

print("=" * 60)
print("Phase C: 类别级性能差异分析 (Hypothesis #3)")
print("=" * 60)

# 加载数据
X, y, feature_names, target_names = load_data()
print(f"\n数据加载完成: {X.shape[0]} 样本, 类别: {target_names}")

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
# C1: 类别级评估
# ============================================
print("\n" + "-" * 40)
print("C1: 类别级性能评估")
print("-" * 40)

models = {
    'KNN': (create_knn(k=best_k), True),
    '决策树': (create_decision_tree(max_depth=best_depth), False),
    '随机森林': (create_random_forest(n_estimators=best_n), False)
}

N_REPEATS = 5
class_names = ['Setosa', 'Versicolor', 'Virginica']

# 存储结果
all_confusion_matrices = {name: [] for name in models.keys()}
all_class_metrics = {name: [] for name in models.keys()}

for repeat in range(N_REPEATS):
    skf = get_stratified_kfold(random_state=42 + repeat)
    
    for name, (model, scale_flag) in models.items():
        fold_preds = {i: [] for i in range(3)}
        fold_true = {i: [] for i in range(3)}
        
        for train_idx, test_idx in skf.split(X, y):
            X_train, X_test = X[train_idx], X[test_idx]
            y_train, y_test = y[train_idx], y[test_idx]
            
            if scale_flag:
                scaler = StandardScaler()
                X_train = scaler.fit_transform(X_train)
                X_test = scaler.transform(X_test)
            
            clf = clone(model)
            clf.fit(X_train, y_train)
            y_pred = clf.predict(X_test)
            
            # 保存每折预测
            for i in range(3):
                mask = (y_test == i)
                fold_preds[i].extend(y_pred[mask])
                fold_true[i].extend(y_test[mask])
        
        # 计算每类指标
        cm = confusion_matrix(y, np.array(fold_preds[0] + fold_preds[1] + fold_preds[2]) 
                              if len(fold_preds[0]) > 0 else y)
        # 直接使用evaluate_model_detailed逻辑
        all_confusion_matrices[name].append(cm)

# 重新使用详细评估
print("\n计算详细类别指标...")
final_metrics = {}

for name, (model, scale_flag) in models.items():
    skf = get_stratified_kfold()
    all_preds = np.zeros_like(y)
    
    for train_idx, test_idx in skf.split(X, y):
        X_train, X_test = X[train_idx], X[test_idx]
        y_train, y_test = y[train_idx], y[test_idx]
        
        if scale_flag:
            scaler = StandardScaler()
            X_train = scaler.fit_transform(X_train)
            X_test = scaler.transform(X_test)
        
        clf = clone(model)
        clf.fit(X_train, y_train)
        all_preds[test_idx] = clf.predict(X_test)
    
    cm = confusion_matrix(y, all_preds)
    report = classification_report(y, all_preds, target_names=class_names, output_dict=True, zero_division=0)
    
    final_metrics[name] = {
        'confusion_matrix': cm,
        'report': report
    }
    
    print(f"\n--- {name} ---")
    print(f"  准确率: {accuracy_score(y, all_preds):.4f}")
    for cls_name in class_names:
        cls_metrics = report[cls_name]
        print(f"  {cls_name}: 精确率={cls_metrics['precision']:.4f}, "
              f"召回率={cls_metrics['recall']:.4f}, "
              f"F1={cls_metrics['f1-score']:.4f}")

# ============================================
# C2: McNemar检验
# ============================================
print("\n" + "-" * 40)
print("C2: McNemar检验 (分类一致性)")
print("-" * 40)

def mcnemar_test(y_true, y_pred1, y_pred2):
    """McNemar检验"""
    n00 = np.sum((y_pred1 == y_true) & (y_pred2 == y_true))
    n01 = np.sum((y_pred1 == y_true) & (y_pred2 != y_true))
    n10 = np.sum((y_pred1 != y_true) & (y_pred2 == y_true))
    n11 = np.sum((y_pred1 != y_true) & (y_pred2 != y_true))
    
    # McNemar's chi-squared statistic (with continuity correction)
    chi2_stat = (abs(n01 - n10) - 1) ** 2 / (n01 + n10) if (n01 + n10) > 0 else 0
    p_value = 1 - chi2.cdf(chi2_stat, 1)
    
    return {
        'n00': n00, 'n01': n01, 'n10': n10, 'n11': n11,
        'chi2': chi2_stat,
        'p_value': p_value,
        '两家都正确': n00,
        '仅模型1正确': n10,
        '仅模型2正确': n01,
        '两家都错误': n11
    }

# 收集所有预测
model_preds = {}
for name, (model, scale_flag) in models.items():
    skf = get_stratified_kfold()
    all_preds = np.zeros_like(y)
    
    for train_idx, test_idx in skf.split(X, y):
        X_train, X_test = X[train_idx], X[test_idx]
        y_train, y_test = y[train_idx], y[test_idx]
        
        if scale_flag:
            scaler = StandardScaler()
            X_train = scaler.fit_transform(X_train)
            X_test = scaler.transform(X_test)
        
        clf = clone(model)
        clf.fit(X_train, y_train)
        all_preds[test_idx] = clf.predict(X_test)
    
    model_preds[name] = all_preds

# 配对McNemar检验
model_names = ['KNN', '决策树', '随机森林']
mcnemar_results = []
for i, name1 in enumerate(model_names):
    for name2 in model_names[i+1:]:
        result = mcnemar_test(y, model_preds[name1], model_preds[name2])
        mcnemar_results.append({
            '对比': f'{name1} vs {name2}',
            '两者正确': result['n00'],
            '仅前者正确': result['n10'],
            '仅后者正确': result['n01'],
            '两者错误': result['n11'],
            'χ²': round(result['chi2'], 4),
            'p值': round(result['p_value'], 6),
            '显著(p<0.05)': '是' if result['p_value'] < 0.05 else '否'
        })
        print(f"\n  {name1} vs {name2}:")
        print(f"    两者正确: {result['n00']}, 仅{name1}正确: {result['n10']}, "
              f"仅{name2}正确: {result['n01']}, 两者错误: {result['n11']}")
        print(f"    McNemar chi2={result['chi2']:.4f}, p={result['p_value']:.6f}")

# 保存McNemar结果
df_mcnemar = pd.DataFrame(mcnemar_results)
save_results_csv(df_mcnemar, 'mcnemar_test_results.csv', 'class_level_performance')

# ============================================
# C3: 可视化
# ============================================
print("\n" + "-" * 40)
print("C3: 可视化")
print("-" * 40)

setup_plot_style()

# 图1-3: 混淆矩阵热力图
for name in model_names:
    cm = final_metrics[name]['confusion_matrix']
    cm_norm = cm.astype('float') / cm.sum(axis=1)[:, np.newaxis]
    
    fig, ax = plt.subplots(figsize=(8, 6))
    sns.heatmap(cm_norm, annot=True, fmt='.2f', cmap='Blues', 
                xticklabels=class_names, yticklabels=class_names,
                ax=ax, vmin=0, vmax=1, cbar_kws={'label': '比例'})
    ax.set_xlabel('预测类别')
    ax.set_ylabel('真实类别')
    ax.set_title(f'{name} — 归一化混淆矩阵', fontweight='bold')
    plt.tight_layout()
    save_figure(fig, f'confusion_matrix_{name}.png', 'class_level_performance')

# 图4: F1-Score对比柱状图
fig, ax = plt.subplots(figsize=(12, 6))
x = np.arange(len(class_names))
width = 0.25
colors = ['#2E86AB', '#A23B72', '#F18F01']

for i, (name, color) in enumerate(zip(model_names, colors)):
    f1_scores = [final_metrics[name]['report'][cls]['f1-score'] for cls in class_names]
    bars = ax.bar(x + i * width, f1_scores, width, label=name, color=color, alpha=0.8)
    
    # 标注数值
    for bar, score in zip(bars, f1_scores):
        ax.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.01,
                f'{score:.3f}', ha='center', va='bottom', fontsize=9)

ax.set_xlabel('类别')
ax.set_ylabel('F1-Score')
ax.set_title('各类别F1-Score对比', fontweight='bold')
ax.set_xticks(x + width)
ax.set_xticklabels(class_names)
ax.legend()
ax.set_ylim(0.8, 1.05)
ax.grid(True, alpha=0.3, axis='y')
plt.tight_layout()
save_figure(fig, 'f1_comparison.png', 'class_level_performance')

# ============================================
# 保存详细结果
# ============================================
print("\n保存详细结果...")
class_level_data = []
for name in model_names:
    for cls_name in class_names:
        cls_metrics = final_metrics[name]['report'][cls_name]
        class_level_data.append({
            '模型': name,
            '类别': cls_name,
            '精确率': round(cls_metrics['precision'], 4),
            '召回率': round(cls_metrics['recall'], 4),
            'F1-Score': round(cls_metrics['f1-score'], 4),
            '支持样本数': cls_metrics['support']
        })

df_class = pd.DataFrame(class_level_data)
save_results_csv(df_class, 'class_level_metrics.csv', 'class_level_performance')

# ============================================
# 假设#3 检验结论
# ============================================
print("\n" + "-" * 40)
print("假设#3 检验结论")
print("-" * 40)

setosa_f1_all_1 = all(
    final_metrics[name]['report']['Setosa']['f1-score'] >= 0.99
    for name in model_names
)

rf_hard_f1 = final_metrics['随机森林']['report']['Versicolor']['f1-score'] + \
             final_metrics['随机森林']['report']['Virginica']['f1-score']
knn_hard_f1 = final_metrics['KNN']['report']['Versicolor']['f1-score'] + \
              final_metrics['KNN']['report']['Virginica']['f1-score']
dt_hard_f1 = final_metrics['决策树']['report']['Versicolor']['f1-score'] + \
             final_metrics['决策树']['report']['Virginica']['f1-score']

print(f"\n  Setosa F1-Score 全部接近1.0: {'✅' if setosa_f1_all_1 else '❌'}")
print(f"  困难类(Versicolor+Virginica) F1总和:")
print(f"    KNN: {knn_hard_f1:.4f}")
print(f"    决策树: {dt_hard_f1:.4f}")
print(f"    随机森林: {rf_hard_f1:.4f}")

if rf_hard_f1 >= knn_hard_f1 and rf_hard_f1 >= dt_hard_f1:
    print(f"\n✅ 假设#3 确认: 随机森林在困难类别上F1最高")
else:
    print(f"\n⚠️ 假设#3 部分否证: 随机森林并非在困难类别上最优")
    best_model = max([('KNN', knn_hard_f1), ('决策树', dt_hard_f1), ('随机森林', rf_hard_f1)], key=lambda x: x[1])
    print(f"   最优模型为: {best_model[0]} (F1总和={best_model[1]:.4f})")

print("\n" + "=" * 60)
print("Phase C 完成!")
print("=" * 60)
