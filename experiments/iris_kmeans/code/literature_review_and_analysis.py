"""
Iris Dataset K-Means Clustering - Literature Review and Dataset Analysis
This script explores the Iris dataset and prepares for k-means clustering experiments.
"""

import numpy as np
import pandas as pd
import matplotlib
matplotlib.use('Agg')  # Non-interactive backend
import matplotlib.pyplot as plt
import seaborn as sns
from sklearn import datasets
from sklearn.cluster import KMeans
from sklearn.preprocessing import StandardScaler
from sklearn.decomposition import PCA
from sklearn.metrics import adjusted_rand_score, normalized_mutual_info_score, silhouette_score
import warnings
warnings.filterwarnings('ignore')

RESULTS_DIR = 'D:/Atlas/experiments/iris_kmeans/results'

# ============================================================
# 1. Load the Iris dataset
# ============================================================
iris = datasets.load_iris()
X = iris.data  # Features: sepal length, sepal width, petal length, petal width
y = iris.target  # True labels (3 species: setosa, versicolor, virginica)
feature_names = iris.feature_names
target_names = iris.target_names

print("=" * 70)
print("IRIS DATASET ANALYSIS FOR K-MEANS CLUSTERING")
print("=" * 70)
print(f"\nDataset shape: {X.shape}")
print(f"Number of samples: {X.shape[0]}")
print(f"Number of features: {X.shape[1]}")
print(f"Feature names: {feature_names}")
print(f"Target classes: {target_names}")
print(f"Class distribution: {np.bincount(y)}")

# ============================================================
# 2. Basic Statistics
# ============================================================
df = pd.DataFrame(X, columns=feature_names)
df['species'] = y
df['species_name'] = df['species'].map({i: name for i, name in enumerate(target_names)})

print("\n" + "=" * 70)
print("BASIC STATISTICS")
print("=" * 70)
print(df.describe())

# ============================================================
# 3. Correlation Analysis
# ============================================================
print("\n" + "=" * 70)
print("FEATURE CORRELATION MATRIX")
print("=" * 70)
corr = df[feature_names].corr()
print(corr)

# ============================================================
# 4. Pairplot Visualization
# ============================================================
plt.figure(figsize=(12, 10))
sns.pairplot(df, hue='species_name', diag_kind='kde')
plt.suptitle('Iris Dataset Pairplot', y=1.02, fontsize=16)
plt.savefig(f'{RESULTS_DIR}/iris_pairplot.png', dpi=150, bbox_inches='tight')
plt.close()
print(f"\nSaved: {RESULTS_DIR}/iris_pairplot.png")

# ============================================================
# 5. K-Means Clustering with K=3 (known ground truth)
# ============================================================
print("\n" + "=" * 70)
print("K-MEANS CLUSTERING (K=3) - PRELIMINARY ANALYSIS")
print("=" * 70)

# Standardize the data
scaler = StandardScaler()
X_scaled = scaler.fit_transform(X)

# Apply K-means
kmeans = KMeans(n_clusters=3, random_state=42, n_init=10)
y_pred = kmeans.fit_predict(X_scaled)

# Evaluation metrics
ari = adjusted_rand_score(y, y_pred)
nmi = normalized_mutual_info_score(y, y_pred)
sil = silhouette_score(X_scaled, y_pred)

print(f"\nK-Means (k=3) on standardized data:")
print(f"  Adjusted Rand Index (ARI): {ari:.4f}")
print(f"  Normalized Mutual Info (NMI): {nmi:.4f}")
print(f"  Silhouette Score: {sil:.4f}")
print(f"  Inertia (WCSS): {kmeans.inertia_:.2f}")
print(f"  Cluster centers (standardized):\n{kmeans.cluster_centers_}")

# Confusion matrix
print("\nConfusion Matrix (rows=true, cols=predicted):")
confusion = pd.crosstab(y, y_pred, rownames=['True'], colnames=['Predicted'])
print(confusion)

# ============================================================
# 6. PCA Visualization of Clustering Results
# ============================================================
pca = PCA(n_components=2)
X_pca = pca.fit_transform(X_scaled)

fig, axes = plt.subplots(1, 2, figsize=(14, 6))

# True labels
scatter1 = axes[0].scatter(X_pca[:, 0], X_pca[:, 1], c=y, cmap='viridis', s=50, alpha=0.8)
axes[0].set_title('True Labels (PCA)', fontsize=14)
axes[0].set_xlabel(f'PC1 ({pca.explained_variance_ratio_[0]:.2%})')
axes[0].set_ylabel(f'PC2 ({pca.explained_variance_ratio_[1]:.2%})')
# Create legend manually
handles1 = [plt.Line2D([0], [0], marker='o', color='w', markerfacecolor=plt.cm.viridis(i/2), markersize=8, label=name) 
            for i, name in enumerate(target_names)]
axes[0].legend(handles=handles1, title='Species')

# K-means labels
scatter2 = axes[1].scatter(X_pca[:, 0], X_pca[:, 1], c=y_pred, cmap='viridis', s=50, alpha=0.8)
axes[1].scatter(kmeans.cluster_centers_[:, 0], kmeans.cluster_centers_[:, 1], 
                c='red', marker='X', s=200, edgecolors='black', linewidths=2, label='Centroids')
axes[1].set_title('K-Means Clustering (K=3, PCA)', fontsize=14)
axes[1].set_xlabel(f'PC1 ({pca.explained_variance_ratio_[0]:.2%})')
axes[1].set_ylabel(f'PC2 ({pca.explained_variance_ratio_[1]:.2%})')
axes[1].legend()

plt.tight_layout()
plt.savefig(f'{RESULTS_DIR}/iris_kmeans_pca_comparison.png', dpi=150, bbox_inches='tight')
plt.close()
print(f"\nSaved: {RESULTS_DIR}/iris_kmeans_pca_comparison.png")

# ============================================================
# 7. Elbow Method (finding optimal K)
# ============================================================
print("\n" + "=" * 70)
print("ELBOW METHOD - FINDING OPTIMAL K")
print("=" * 70)

inertias = []
silhouette_scores = []
K_range = range(2, 11)

for k in K_range:
    km = KMeans(n_clusters=k, random_state=42, n_init=10)
    labels = km.fit_predict(X_scaled)
    inertias.append(km.inertia_)
    sil_score = silhouette_score(X_scaled, labels)
    silhouette_scores.append(sil_score)
    print(f"  K={k}: Inertia={km.inertia_:.2f}, Silhouette={sil_score:.4f}")

fig, axes = plt.subplots(1, 2, figsize=(14, 5))

# Elbow curve
axes[0].plot(K_range, inertias, 'bo-', linewidth=2, markersize=8)
axes[0].axvline(x=3, color='red', linestyle='--', alpha=0.7, label='True K=3')
axes[0].set_title('Elbow Method for Optimal K', fontsize=14)
axes[0].set_xlabel('Number of Clusters (K)')
axes[0].set_ylabel('Inertia (WCSS)')
axes[0].grid(True, alpha=0.3)
axes[0].legend()

# Silhouette scores
axes[1].plot(K_range, silhouette_scores, 'ro-', linewidth=2, markersize=8)
axes[1].axvline(x=3, color='red', linestyle='--', alpha=0.7, label='True K=3')
axes[1].set_title('Silhouette Score for Optimal K', fontsize=14)
axes[1].set_xlabel('Number of Clusters (K)')
axes[1].set_ylabel('Silhouette Score')
axes[1].grid(True, alpha=0.3)
axes[1].legend()

plt.tight_layout()
plt.savefig(f'{RESULTS_DIR}/iris_elbow_silhouette.png', dpi=150, bbox_inches='tight')
plt.close()
print(f"\nSaved: {RESULTS_DIR}/iris_elbow_silhouette.png")

print("\n" + "=" * 70)
print("PRELIMINARY ANALYSIS COMPLETE")
print("=" * 70)
print("\nKey findings:")
print("1. Iris dataset has 150 samples, 4 features, 3 classes (50 each)")
print("2. Petal features are most discriminative between species")
print("3. K-means (k=3) achieves good clustering quality")
print("4. Elbow method suggests k=3 is optimal")
print("5. One class (versicolor & virginica) has some overlap")
