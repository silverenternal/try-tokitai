# Experiment Design: 机器学习模型对比 — KNN、决策树、随机森林在Iris数据集上的比较

**Phase: Experiment Design**
**Date: 2026-05-28**

---

## 1. 假设选择

### 选中的假设：假设 #1（最高优先级）— 超参数敏感性差异

**理由：**
1. 假设#1是所有后续假设的基础——它为假设#2和#3提供最优超参数配置
2. 新颖性高：现有教学示例大多使用默认参数，缺乏系统性的超参数影响分析
3. 可行性最高：仅需scikit-learn内置Iris数据集，计算量适中
4. 产出价值大：提供可操作的超参数调优指南

### 整体实验策略：5个假设依次执行

```
Phase A: 假设#1 (超参数敏感性) → 输出最优超参数
Phase B: 假设#2 (性能等价性) → 使用Phase A的最优参数
Phase C: 假设#3 (类别级差异) → 使用Phase A的最优参数
Phase D: 假设#4 (规模敏感性) → 独立执行
Phase E: 假设#5 (特征鲁棒性) → 独立执行
```

---

## 2. 数据

### 2.1 数据集：scikit-learn内置Iris数据集

**来源：** `sklearn.datasets.load_iris()`
**特性：**

| 属性 | 值 |
|------|-----|
| 样本数 | 150 |
| 特征数 | 4 |
| 特征名称 | sepal length (cm), sepal width (cm), petal length (cm), petal width (cm) |
| 类别数 | 3 (0=Setosa, 1=Versicolor, 2=Virginica) |
| 每类样本 | 50 (完全平衡) |
| 缺失值 | 无 |
| 特征类型 | 全部连续数值型 |

**数据预处理：**
- 对KNN：使用 `StandardScaler` 对特征进行标准化（因为KNN依赖距离度量）
- 对决策树和随机森林：不需要标准化（树模型对特征尺度不敏感）
- 注意：为公平对比，标准化仅应用于KNN的输入，不影响决策树和随机森林

### 2.2 数据划分策略

使用 **分层10折交叉验证 (Stratified 10-Fold Cross-Validation)**：
- `StratifiedKFold(n_splits=10, shuffle=True, random_state=42)`
- 确保每一折中各类别比例与原始数据一致（每折约15样本，每类5个）
- 每次实验使用固定随机种子以保证可复现性

---

## 3. 基线方法

| 模型 | scikit-learn类 | 默认参数 | 待调优超参数 |
|------|---------------|---------|-------------|
| KNN | `KNeighborsClassifier` | n_neighbors=5, p=2 | n_neighbors (k值) |
| 决策树 | `DecisionTreeClassifier` | criterion='gini', max_depth=None | max_depth |
| 随机森林 | `RandomForestClassifier` | n_estimators=100, max_depth=None | n_estimators |

**注意：** 不对其他参数（如距离度量、分裂准则、最小样本分裂数等）进行调优，以保持实验聚焦。

---

## 4. 评估指标

### 4.1 核心指标
| 指标 | 定义 | 适用场景 |
|------|------|---------|
| **准确率 (Accuracy)** | (TP+TN)/(TP+TN+FP+FN) | 假设#1, #2, #4, #5 |
| **精确率 (Precision)** | TP/(TP+FP) (macro平均) | 假设#3 |
| **召回率 (Recall)** | TP/(TP+FN) (macro平均) | 假设#3 |
| **F1-Score** | 2×P×R/(P+R) (macro平均) | 假设#3 |
| **混淆矩阵** | 每类预测vs真实计数 | 假设#3 |

### 4.2 统计指标
| 检验 | 用途 | 适用假设 |
|------|------|---------|
| **配对t检验** | 两两模型准确率差异显著性 | 假设#2 |
| **Bonferroni校正** | 多重比较校正 (3次比较) | 假设#2 |
| **Cohen's d** | 效应量 | 假设#2 |
| **McNemar检验** | 分类一致性检验 | 假设#3 |

### 4.3 辅助指标
| 指标 | 用途 |
|------|------|
| **训练时间** | 评估计算成本 |
| **预测时间** | 评估推理效率 |
| **标准差/方差** | 评估模型稳定性 |

---

## 5. 实验协议（逐步执行）

### Phase A: 假设#1 — 超参数敏感性差异

**目标：** 分析三个模型对核心超参数的敏感性，确定最优超参数

**步骤 A1：KNN超参数扫描**
```
参数范围: k = [1, 3, 5, 7, 9, 11, 15, 21]
其他参数: metric='euclidean', weights='uniform'
流程:
  for each k in [1, 3, 5, 7, 9, 11, 15, 21]:
    重复5次:
      10折分层交叉验证
      记录每次的准确率均值和标准差
输出: k值 vs 准确率曲线, 最优k值
```

**步骤 A2：决策树超参数扫描**
```
参数范围: max_depth = [1, 2, 3, 4, 5, 6, 8, 10, None]
其他参数: criterion='gini', min_samples_split=2
流程:
  for each depth in [1, 2, 3, 4, 5, 6, 8, 10, None]:
    重复5次:
      10折分层交叉验证
      记录每次的准确率均值和标准差
输出: max_depth vs 准确率曲线, 最优max_depth
```

**步骤 A3：随机森林超参数扫描**
```
参数范围: n_estimators = [1, 5, 10, 20, 50, 100, 200, 500]
其他参数: max_depth=None, criterion='gini'
流程:
  for each n in [1, 5, 10, 20, 50, 100, 200, 500]:
    重复5次:
      10折分层交叉验证
      记录每次的准确率均值和标准差
输出: n_estimators vs 准确率曲线, 最优n_estimators
```

**步骤 A4：可视化与输出**
- 绘制3张超参数-准确率曲线图（含误差带）
- 确定每个模型的最优超参数配置
- 保存到 `results/hyperparameter_sensitivity/`

---

### Phase B: 假设#2 — 模型性能等价性

**目标：** 检验最优参数下三个模型的性能是否存在统计显著差异

**步骤 B1：最优配置下的对比实验**
```
配置:
  KNN: k=最优值 (来自A1), metric='euclidean'
  DT: max_depth=最优值 (来自A2), criterion='gini'
  RF: n_estimators=最优值 (来自A3), max_depth=None

流程:
  重复10次:
    10折分层交叉验证 (每次不同随机种子)
    记录每个模型的10个准确率 (每折一个)
  得到: 每个模型 10次×10折 = 100个准确率样本
```

**步骤 B2：统计检验**
```
- 正态性检验: Shapiro-Wilk test
- 三组比较: 重复测量方差分析 (Repeated Measures ANOVA)
- 两两比较: 配对t检验 (3次比较)
- 多重比较校正: Bonferroni (α' = 0.05/3 ≈ 0.0167)
- 效应量: Cohen's d
- 非参数验证: Friedman检验 + Wilcoxon符号秩检验
```

**步骤 B3：可视化与输出**
- 箱线图：三个模型的准确率分布
- 配对散点图：展示两两模型的准确率对应关系
- 保存到 `results/performance_equivalence/`

---

### Phase C: 假设#3 — 类别级性能差异

**目标：** 分析每个模型在三个类别上的性能差异

**步骤 C1：类别级评估**
```
配置: 使用Phase A得到的最优参数

流程:
  10折分层交叉验证
  每折计算:
    - 混淆矩阵 (3×3)
    - 每类的精确率、召回率、F1-Score
    - macro平均F1-Score
  重复5次 → 取平均
```

**步骤 C2：McNemar检验**
```
- 对每对模型进行McNemar检验
- 检验: 两个模型在哪些样本上分类结果不一致
- 统计不一致样本的分布
```

**步骤 C3：可视化与输出**
- 归一化混淆矩阵热力图 (3个模型×3类)
- 每类F1-Score柱状图对比
- 保存到 `results/class_level_performance/`

---

### Phase D: 假设#4 — 训练集规模敏感性

**目标：** 分析训练集规模对三个模型性能的影响

**步骤 D1：学习曲线实验**
```
训练集比例: [10%, 20%, 30%, 40%, 50%, 70%, 85%, 100%]
对应样本数: [15, 30, 45, 60, 75, 105, 128, 150]

流程:
  for each ratio in train_sizes:
    重复10次:
      分层采样对应比例的样本作为训练集
      剩余作为测试集
      训练并评估三个模型（使用最优参数）
      记录测试准确率
  → 每个比例下10个准确率样本/模型
```

**步骤 D2：可视化与输出**
- 学习曲线图 (训练规模 vs 准确率，含误差带)
- 标注"小样本阈值"：准确率开始显著下降的临界点
- 保存到 `results/learning_curves/`

---

### Phase E: 假设#5 — 特征子集鲁棒性

**目标：** 分析不同特征子集对三个模型性能的影响

**步骤 E1：特征子集实验**
```
特征子集方案:
  F1: [0, 1, 2, 3] — 全部4个特征 (sepal+petal)
  F2: [2, 3] — 仅花瓣特征 (petal length, petal width)
  F3: [0, 1] — 仅花萼特征 (sepal length, sepal width)
  F4: [0, 2] — 花萼长度 + 花瓣长度
  F5: [1, 3] — 花萼宽度 + 花瓣宽度

流程:
  for each feature subset in [F1, F2, F3, F4, F5]:
    重复5次:
      10折分层交叉验证
      训练并评估三个模型（使用最优参数）
      记录准确率、F1-Score
  → 每个子集下50个准确率样本/模型
```

**步骤 E2：可视化与输出**
- 特征子集 vs 准确率的组间对比柱状图
- 每个特征子集下三个模型的性能比较
- 保存到 `results/feature_robustness/`

---

## 6. 混杂因素控制

| 潜在混杂因素 | 控制措施 |
|-------------|---------|
| 数据划分随机性 | 固定随机种子 (random_state=42)，重复实验取平均 |
| 交叉验证折数 | 统一使用10折分层交叉验证 |
| 超参数差异 | Phase A中系统扫描，非手动选择 |
| 实现差异 | 全部使用scikit-learn统一实现 |
| 特征缩放 | KNN使用标准化，树模型使用原始值（各自最优实践） |
| 随机森林随机性 | 固定random_state，多次重复取均值 |

---

## 7. 实验环境

| 组件 | 规格 |
|------|------|
| Python版本 | 3.9+ |
| scikit-learn | 1.0+ |
| 依赖库 | numpy, pandas, matplotlib, seaborn, scipy |
| 运行平台 | Windows |

**依赖安装：**
```bash
pip install scikit-learn numpy pandas matplotlib seaborn scipy
```

---

## 8. 输出产物

### 8.1 代码文件
| 文件 | 内容 |
|------|------|
| `code/01_hyperparameter_sensitivity.py` | Phase A: 超参数敏感性分析 |
| `code/02_performance_equivalence.py` | Phase B: 性能等价性检验 |
| `code/03_class_level_analysis.py` | Phase C: 类别级性能分析 |
| `code/04_learning_curves.py` | Phase D: 学习曲线实验 |
| `code/05_feature_robustness.py` | Phase E: 特征子集鲁棒性 |
| `code/utils.py` | 通用工具函数（数据加载、评估、绘图） |

### 8.2 结果目录
```
results/
├── hyperparameter_sensitivity/
│   ├── knn_k_sensitivity.png
│   ├── dt_depth_sensitivity.png
│   └── rf_estimators_sensitivity.png
├── performance_equivalence/
│   ├── accuracy_boxplot.png
│   ├── paired_scatter.png
│   └── statistical_test_results.csv
├── class_level_performance/
│   ├── confusion_matrix_knn.png
│   ├── confusion_matrix_dt.png
│   ├── confusion_matrix_rf.png
│   └── f1_comparison.png
├── learning_curves/
│   └── learning_curves_comparison.png
└── feature_robustness/
    └── feature_subset_comparison.png
```

### 8.3 数据文件
```
results/
├── hyperparameter_sensitivity_results.csv
├── performance_equivalence_results.csv
├── class_level_results.csv
├── learning_curve_results.csv
└── feature_robustness_results.csv
```

---

## 9. 预期时间线

| Phase | 实验内容 | 预计运行时间 | 代码行数 |
|-------|---------|------------|---------|
| A | 超参数敏感性 | ~2分钟 | ~150行 |
| B | 性能等价性 | ~1分钟 | ~100行 |
| C | 类别级分析 | ~1分钟 | ~100行 |
| D | 学习曲线 | ~2分钟 | ~120行 |
| E | 特征鲁棒性 | ~1分钟 | ~100行 |
| **总计** | | **~7分钟** | **~570行** |

---

## 10. 成功标准

| 假设 | 确认标准 | 否证标准 |
|------|---------|---------|
| #1 | KNN倒U型峰在k=3~5；DT在depth=3~4饱和；RF在n>50稳定 | 单调变化或持续上升 |
| #2 | 三模型配对t检验p>0.05 (Bonferroni校正后) | 任一对比p<0.0167 |
| #3 | Setosa F1=1.0；RF在困难类F1最高 | KNN在困难类最优 |
| #4 | 训练集减至30时KNN下降>8%，DT~5%，RF<4% | 下降幅度相近 |
| #5 | 特征减少时DT降幅最小，RF降幅最大 | 降幅一致 |
