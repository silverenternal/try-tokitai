#!/bin/bash
# 实验数据收集脚本
# 用于收集 HybridGapDetector 的性能指标和 API 成本数据

set -e

EXPERIMENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$EXPERIMENT_DIR")"
DATA_DIR="$EXPERIMENT_DIR/data"
RESULTS_DIR="$EXPERIMENT_DIR/results"

echo "========================================"
echo "  Tokitai 实验数据收集脚本"
echo "========================================"
echo ""
echo "📁 项目根目录：$PROJECT_ROOT"
echo "📊 数据目录：$DATA_DIR"
echo "📈 结果目录：$RESULTS_DIR"
echo ""

# 创建数据目录
mkdir -p "$DATA_DIR"
mkdir -p "$RESULTS_DIR"

# 实验配置
EXPERIMENT_NAME="${1:-hybrid_gap_detector_benchmark}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="$DATA_DIR/${EXPERIMENT_NAME}_${TIMESTAMP}.log"
METRICS_FILE="$DATA_DIR/${EXPERIMENT_NAME}_${TIMESTAMP}_metrics.json"

echo "🏷️  实验名称：$EXPERIMENT_NAME"
echo "📝 日志文件：$LOG_FILE"
echo "📊 指标文件：$METRICS_FILE"
echo ""

# 运行基准测试
echo "🚀 开始运行基准测试..."
echo ""

cd "$PROJECT_ROOT"

# 运行 HybridGapDetector 基准测试
cargo bench --bench hybrid_gap_detector_bench 2>&1 | tee "$LOG_FILE"

# 提取关键指标
echo ""
echo "📊 提取实验指标..."

# 解析基准测试结果（示例）
cat > "$METRICS_FILE" << EOF
{
    "experiment_name": "$EXPERIMENT_NAME",
    "timestamp": "$(date -Iseconds)",
    "git_commit": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
    "metrics": {
        "detection_latency_ms": null,
        "api_calls_per_cycle": null,
        "api_cost_usd": null,
        "cache_hit_rate": null,
        "gaps_detected": null
    },
    "configuration": {
        "statistical_threshold": 0.5,
        "causal_min_priority": 6,
        "max_causal_analyses": 5,
        "api_budget_per_cycle": 0.5
    },
    "notes": ""
}
EOF

echo ""
echo "✅ 实验数据收集完成！"
echo ""
echo "📁 数据文件位置:"
echo "   - 原始日志：$LOG_FILE"
echo "   - 指标 JSON: $METRICS_FILE"
echo ""
echo "📈 下一步:"
echo "   1. 编辑 $METRICS_FILE 填写实验结果"
echo "   2. 运行分析脚本生成图表"
echo "   3. 将结果添加到论文实验章节"
echo ""
