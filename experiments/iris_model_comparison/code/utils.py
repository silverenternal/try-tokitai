"""
utils.py — 通用工具函数
用于机器学习模型对比实验（Iris数据集）
"""

import numpy as np
import pandas as pd
import matplotlib
matplotlib.use('Agg')  # 非交互式后端
import matplotlib.pyplot as plt
import seaborn as sns
from sklearn.datasets import load_iris
from sklearn.model_selection import StratifiedKFold
from sklearn.preprocessing import StandardScaler
from sklearn.neighbors import KNeighborsClassifier
from sklearn.tree import DecisionTreeClassifier
from sklearn.ensemble import RandomForestClassifier
from sklearn.metrics import (
    accuracy_score, precision_score, recall_score, f1_score,
    confusion_matrix, classification_report
)
import os
import warnings
warnings.filterwarnings('ignore')

# ========== 全局配置 ==========
RANDOM_STATE = 42
N_SPLITS = 10
N_REPEATS = 5  # 重复次数
RESULTS_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), 'results')
CODE_DIR = os.path.dirname(os.path.abspath(__file__))

# ========== 数据加载 ==========

def load_data():
    """加载Iris数据集"""
    iris = load_iris()
    X, y = iris.data, iris.target
    feature_names = iris.feature_names
    target_names = iris.target_names
    return X, y, feature_names, target_names


def get_stratified_kfold(random_state=RANDOM_STATE):
    """返回分层K折交叉验证对象"""
    return StratifiedKFold(n_splits=N_SPLITS, shuffle=True, random_state=random_state)


# ========== 模型工厂 ==========

def create_knn(k=5):
    """创建KNN分类器"""
    return KNeighborsClassifier(n_neighbors=k, metric='euclidean', weights='uniform')


def create_decision_tree(max_depth=None):
    """创建决策树分类器"""
    return DecisionTreeClassifier(criterion='gini', max_depth=max_depth, random_state=RANDOM_STATE)


def create_random_forest(n_estimators=100):
    """创建随机森林分类器"""
    return RandomForestClassifier(
        n_estimators=n_estimators, criterion='gini', max_depth=None,
        random_state=RANDOM_STATE, n_jobs=-1
    )


# ========== 评估函数 ==========

def evaluate_model_cv(model, X, y, n_repeats=N_REPEATS, scale=False):
    """
    使用重复分层K折交叉验证评估模型
    
    参数:
        model: 分类器对象
        X, y: 特征和标签
        n_repeats: 重复次数
        scale: 是否对特征进行标准化（KNN需要）
    
    返回:
        scores: 所有折的准确率列表 (n_repeats * n_splits)
        mean_score: 平均准确率
        std_score: 标准差
    """
    all_scores = []
    
    for repeat in range(n_repeats):
        skf = get_stratified_kfold(random_state=RANDOM_STATE + repeat)
        
        for train_idx, test_idx in skf.split(X, y):
            X_train, X_test = X[train_idx], X[test_idx]
            y_train, y_test = y[train_idx], y[test_idx]
            
            if scale:
                scaler = StandardScaler()
                X_train = scaler.fit_transform(X_train)
                X_test = scaler.transform(X_test)
            
            # 克隆模型避免状态污染
            from sklearn.base import clone
            clf = clone(model)
            clf.fit(X_train, y_train)
            y_pred = clf.predict(X_test)
            all_scores.append(accuracy_score(y_test, y_pred))
    
    return np.array(all_scores), np.mean(all_scores), np.std(all_scores)


def evaluate_model_detailed(model, X, y, scale=False):
    """
    详细评估模型：返回准确率、精确率、召回率、F1、混淆矩阵
    
    返回:
        metrics: dict 包含各项指标
    """
    skf = get_stratified_kfold()
    all_preds = np.zeros_like(y)
    all_true = np.zeros_like(y)
    
    for train_idx, test_idx in skf.split(X, y):
        X_train, X_test = X[train_idx], X[test_idx]
        y_train, y_test = y[train_idx], y[test_idx]
        
        if scale:
            scaler = StandardScaler()
            X_train = scaler.fit_transform(X_train)
            X_test = scaler.transform(X_test)
        
        from sklearn.base import clone
        clf = clone(model)
        clf.fit(X_train, y_train)
        all_preds[test_idx] = clf.predict(X_test)
        all_true[test_idx] = y_test
    
    metrics = {
        'accuracy': accuracy_score(all_true, all_preds),
        'precision': precision_score(all_true, all_preds, average='macro', zero_division=0),
        'recall': recall_score(all_true, all_preds, average='macro', zero_division=0),
        'f1': f1_score(all_true, all_preds, average='macro', zero_division=0),
        'confusion_matrix': confusion_matrix(all_true, all_preds),
        'predictions': all_preds,
        'true_labels': all_true
    }
    return metrics


# ========== 可视化函数 ==========

def setup_plot_style():
    """设置统一的绘图样式"""
    plt.rcParams.update({
        'figure.figsize': (10, 6),
        'font.size': 12,
        'axes.titlesize': 14,
        'axes.labelsize': 12,
        'xtick.labelsize': 10,
        'ytick.labelsize': 10,
        'legend.fontsize': 11,
        'figure.dpi': 150,
        'savefig.dpi': 150,
        'font.family': 'sans-serif',
    })
    # 尝试设置中文字体
    try:
        plt.rcParams['font.sans-serif'] = ['SimHei', 'DejaVu Sans', 'Arial']
        plt.rcParams['axes.unicode_minus'] = False
    except:
        pass


def save_figure(fig, filename, subdir=''):
    """保存图表到results目录"""
    save_dir = os.path.join(RESULTS_DIR, subdir)
    os.makedirs(save_dir, exist_ok=True)
    filepath = os.path.join(save_dir, filename)
    fig.savefig(filepath)
    plt.close(fig)
    print(f"  [保存] {filepath}")
    return filepath


def save_results_csv(data, filename, subdir=''):
    """保存结果到CSV文件"""
    save_dir = os.path.join(RESULTS_DIR, subdir)
    os.makedirs(save_dir, exist_ok=True)
    filepath = os.path.join(save_dir, filename)
    if isinstance(data, pd.DataFrame):
        data.to_csv(filepath, index=False)
    else:
        pd.DataFrame(data).to_csv(filepath, index=False)
    print(f"  [保存] {filepath}")
    return filepath


def plot_hyperparameter_sensitivity(param_values, mean_scores, std_scores, 
                                     param_name, model_name, color='steelblue'):
    """绘制超参数敏感性曲线"""
    setup_plot_style()
    fig, ax = plt.subplots(figsize=(10, 6))
    
    x = range(len(param_values))
    ax.errorbar(x, mean_scores, yerr=std_scores, fmt='o-', color=color, 
                capsize=5, capthick=2, linewidth=2, markersize=8, 
                markerfacecolor=color, markeredgecolor='white', markeredgewidth=2)
    
    # 标注最优值
    best_idx = np.argmax(mean_scores)
    ax.annotate(f'最优: {param_values[best_idx]}\n准确率: {mean_scores[best_idx]:.3f}',
                xy=(best_idx, mean_scores[best_idx]),
                xytext=(best_idx + 0.3, mean_scores[best_idx] - 0.02),
                arrowprops=dict(arrowstyle='->', color='red', lw=1.5),
                fontsize=11, color='red', fontweight='bold')
    
    ax.set_xticks(x)
    ax.set_xticklabels([str(v) for v in param_values])
    ax.set_xlabel(param_name)
    ax.set_ylabel('10折交叉验证准确率')
    ax.set_title(f'{model_name} — {param_name} 对准确率的影响', fontweight='bold')
    ax.grid(True, alpha=0.3)
    ax.set_ylim(min(mean_scores) - 0.05, max(mean_scores) + 0.05)
    
    # 添加参考线
    ax.axhline(y=mean_scores[best_idx], color='red', linestyle='--', alpha=0.3)
    
    return fig
