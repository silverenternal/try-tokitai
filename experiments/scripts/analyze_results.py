#!/usr/bin/env python3
"""
实验结果分析脚本

用于分析基准测试结果，生成统计报告和可视化图表

使用方法:
    python analyze_results.py
"""

import json
import os
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Optional
import statistics


# ============================================================================
# 配置
# ============================================================================

EXPERIMENTS_DIR = Path(__file__).parent.parent
LOGS_DIR = EXPERIMENTS_DIR / "logs"
ANALYSIS_DIR = EXPERIMENTS_DIR / "analysis"
VISUALIZATIONS_DIR = ANALYSIS_DIR / "visualizations"

# 确保输出目录存在
ANALYSIS_DIR.mkdir(parents=True, exist_ok=True)
VISUALIZATIONS_DIR.mkdir(parents=True, exist_ok=True)


# ============================================================================
# 数据加载
# ============================================================================

def load_all_logs() -> Dict[str, List[Dict[str, Any]]]:
    """加载所有实验组的日志"""
    all_logs = {}
    
    for group_dir in LOGS_DIR.iterdir():
        if not group_dir.is_dir():
            continue
        
        group_name = group_dir.name.replace("_", " ").title()
        logs = []
        
        for log_file in group_dir.glob("task_logs_*.jsonl"):
            with open(log_file, "r", encoding="utf-8") as f:
                for line in f:
                    logs.append(json.loads(line))
        
        if logs:
            all_logs[group_name] = logs
    
    return all_logs


def load_evolution_logs() -> Dict[str, List[Dict[str, Any]]]:
    """加载所有实验组的自进化日志"""
    all_logs = {}
    
    for group_dir in LOGS_DIR.iterdir():
        if not group_dir.is_dir():
            continue
        
        group_name = group_dir.name.replace("_", " ").title()
        logs = []
        
        for log_file in group_dir.glob("evolution_logs_*.jsonl"):
            with open(log_file, "r", encoding="utf-8") as f:
                for line in f:
                    logs.append(json.loads(line))
        
        if logs:
            all_logs[group_name] = logs
    
    return all_logs


# ============================================================================
# 统计分析
# ============================================================================

def calculate_group_stats(logs: List[Dict[str, Any]]) -> Dict[str, Any]:
    """计算单组的统计指标"""
    if not logs:
        return {}
    
    # 提取指标
    success_rates = [1 if log["execution"]["success"] else 0 for log in logs]
    tool_calls = [log["execution"]["total_tool_calls"] for log in logs]
    execution_times = [log["execution"]["execution_time_ms"] for log in logs]
    satisfactions = [log["execution"]["user_satisfaction"] for log in logs]
    
    # 按难度分组统计
    by_difficulty = {}
    for log in logs:
        diff = log["difficulty"]
        if diff not in by_difficulty:
            by_difficulty[diff] = {"success": 0, "total": 0}
        by_difficulty[diff]["total"] += 1
        if log["execution"]["success"]:
            by_difficulty[diff]["success"] += 1
    
    difficulty_stats = {
        diff: stats["success"] / stats["total"] if stats["total"] > 0 else 0
        for diff, stats in by_difficulty.items()
    }
    
    # 按类别分组统计
    by_category = {}
    for log in logs:
        cat = log["category"]
        if cat not in by_category:
            by_category[cat] = {"success": 0, "total": 0, "tool_calls": 0}
        by_category[cat]["total"] += 1
        if log["execution"]["success"]:
            by_category[cat]["success"] += 1
        by_category[cat]["tool_calls"] += log["execution"]["total_tool_calls"]
    
    category_stats = {
        cat: {
            "success_rate": stats["success"] / stats["total"] if stats["total"] > 0 else 0,
            "avg_tool_calls": stats["tool_calls"] / stats["total"] if stats["total"] > 0 else 0
        }
        for cat, stats in by_category.items()
    }
    
    return {
        "total_tasks": len(logs),
        "success_rate": statistics.mean(success_rates),
        "success_rate_std": statistics.stdev(success_rates) if len(success_rates) > 1 else 0,
        "avg_tool_calls": statistics.mean(tool_calls),
        "tool_calls_std": statistics.stdev(tool_calls) if len(tool_calls) > 1 else 0,
        "avg_execution_time_ms": statistics.mean(execution_times),
        "execution_time_std": statistics.stdev(execution_times) if len(execution_times) > 1 else 0,
        "avg_satisfaction": statistics.mean(satisfactions),
        "satisfaction_std": statistics.stdev(satisfactions) if len(satisfactions) > 1 else 0,
        "difficulty_stats": difficulty_stats,
        "category_stats": category_stats,
        "median_tool_calls": statistics.median(tool_calls),
        "median_execution_time_ms": statistics.median(execution_times),
    }


def calculate_evolution_stats(logs: List[Dict[str, Any]]) -> Dict[str, Any]:
    """计算自进化统计指标"""
    if not logs:
        return {}
    
    total_gaps = sum(len(log["gaps_detected"]) for log in logs)
    total_created = sum(
        sum(1 for a in log["actions_taken"] if a["action_type"] == "create_tool")
        for log in logs
    )
    total_optimized = sum(
        sum(1 for a in log["actions_taken"] if a["action_type"] == "optimize_tool")
        for log in logs
    )
    total_api_calls = sum(log["metrics"]["api_calls"] for log in logs)
    total_api_cost = sum(log["metrics"]["api_cost_usd"] for log in logs)
    
    return {
        "total_cycles": len(logs),
        "total_gaps_detected": total_gaps,
        "total_tools_created": total_created,
        "total_tools_optimized": total_optimized,
        "total_api_calls": total_api_calls,
        "total_api_cost_usd": total_api_cost,
        "avg_api_calls_per_cycle": total_api_calls / len(logs) if logs else 0,
        "avg_cost_per_cycle": total_api_cost / len(logs) if logs else 0,
    }


def compare_groups(all_stats: Dict[str, Dict[str, Any]]) -> Dict[str, Any]:
    """比较多组结果"""
    if len(all_stats) < 2:
        return {}
    
    # 以 Control 组为基线
    baseline = all_stats.get("Control", {})
    if not baseline:
        return {}
    
    comparisons = {}
    
    for group, stats in all_stats.items():
        if group == "Control":
            continue
        
        # 计算相对提升
        improvement = {
            "success_rate_improvement": (
                (stats["success_rate"] - baseline["success_rate"]) / baseline["success_rate"] * 100
                if baseline["success_rate"] > 0 else 0
            ),
            "tool_calls_reduction": (
                (baseline["avg_tool_calls"] - stats["avg_tool_calls"]) / baseline["avg_tool_calls"] * 100
                if baseline["avg_tool_calls"] > 0 else 0
            ),
            "execution_time_change": (
                (stats["avg_execution_time_ms"] - baseline["avg_execution_time_ms"]) / baseline["avg_execution_time_ms"] * 100
                if baseline["avg_execution_time_ms"] > 0 else 0
            ),
            "satisfaction_improvement": (
                (stats["avg_satisfaction"] - baseline["avg_satisfaction"]) / baseline["avg_satisfaction"] * 100
                if baseline["avg_satisfaction"] > 0 else 0
            ),
        }
        
        comparisons[group] = improvement
    
    return comparisons


# ============================================================================
# 报告生成
# ============================================================================

def generate_comparison_report(
    all_stats: Dict[str, Dict[str, Any]],
    comparisons: Dict[str, Dict[str, Any]]
) -> str:
    """生成对比报告"""
    report = []
    report.append("# 实验结果对比报告\n")
    report.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
    report.append("\n")
    
    # 总体统计
    report.append("## 总体统计\n\n")
    report.append("| 组别 | 任务数 | 成功率 | 平均工具调用 | 平均执行时间 (ms) | 平均满意度 |\n")
    report.append("|------|--------|--------|--------------|-------------------|------------|\n")
    
    for group, stats in all_stats.items():
        report.append(
            f"| {group} | {stats['total_tasks']} | "
            f"{stats['success_rate']:.1%} | {stats['avg_tool_calls']:.1f} | "
            f"{stats['avg_execution_time_ms']:.0f} | {stats['avg_satisfaction']:.1f} |\n"
        )
    
    report.append("\n")
    
    # 改进对比
    report.append("## 相对基线改进\n\n")
    report.append("| 组别 | 成功率提升 | 工具调用减少 | 执行时间变化 | 满意度提升 |\n")
    report.append("|------|------------|--------------|--------------|------------|\n")
    
    for group, improvement in comparisons.items():
        report.append(
            f"| {group} | {improvement['success_rate_improvement']:+.1f}% | "
            f"{improvement['tool_calls_reduction']:+.1f}% | "
            f"{improvement['execution_time_change']:+.1f}% | "
            f"{improvement['satisfaction_improvement']:+.1f}% |\n"
        )
    
    report.append("\n")
    
    # 按难度分析
    report.append("## 按难度分析\n\n")
    report.append("| 组别 | 简单 | 中等 | 困难 |\n")
    report.append("|------|------|------|------|\n")
    
    for group, stats in all_stats.items():
        diff_stats = stats.get("difficulty_stats", {})
        easy = diff_stats.get("easy", 0)
        medium = diff_stats.get("medium", 0)
        hard = diff_stats.get("hard", 0)
        report.append(
            f"| {group} | {easy:.1%} | {medium:.1%} | {hard:.1%} |\n"
        )
    
    report.append("\n")
    
    # 自进化统计（如果有）
    evo_logs = load_evolution_logs()
    if evo_logs:
        report.append("## 自进化统计\n\n")
        report.append("| 组别 | 进化周期 | 检测缺口 | 创建工具 | API 成本 ($) |\n")
        report.append("|------|----------|----------|----------|------------|\n")
        
        for group, logs in evo_logs.items():
            evo_stats = calculate_evolution_stats(logs)
            report.append(
                f"| {group} | {evo_stats['total_cycles']} | "
                f"{evo_stats['total_gaps_detected']} | "
                f"{evo_stats['total_tools_created']} | "
                f"{evo_stats['total_api_cost_usd']:.2f} |\n"
            )
        
        report.append("\n")
    
    return "".join(report)


def save_report(report: str, filename: str = "comparison_report.md"):
    """保存报告到文件"""
    output_file = ANALYSIS_DIR / filename
    with open(output_file, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"报告已保存到：{output_file}")


def save_json_stats(
    all_stats: Dict[str, Dict[str, Any]],
    comparisons: Dict[str, Dict[str, Any]]
):
    """保存 JSON 格式统计"""
    output_file = ANALYSIS_DIR / "comparison_results.json"
    data = {
        "generated_at": datetime.now().isoformat(),
        "group_stats": all_stats,
        "comparisons": comparisons
    }
    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
    print(f"JSON 统计已保存到：{output_file}")


# ============================================================================
# 可视化（ASCII 图表）
# ============================================================================

def generate_ascii_bar_chart(
    data: Dict[str, float],
    title: str,
    width: int = 50,
    max_value: Optional[float] = None
) -> str:
    """生成 ASCII 条形图"""
    if not data:
        return ""
    
    lines = []
    lines.append(f"\n{title}\n")
    lines.append("-" * 60 + "\n")
    
    if max_value is None:
        max_value = max(data.values())
    
    for label, value in sorted(data.items(), key=lambda x: x[1], reverse=True):
        bar_length = int((value / max_value) * width) if max_value > 0 else 0
        bar = "█" * bar_length + "░" * (width - bar_length)
        lines.append(f"{label:<15} |{bar}| {value:.2f}\n")
    
    lines.append("\n")
    return "".join(lines)


def generate_visualizations(all_stats: Dict[str, Dict[str, Any]]):
    """生成可视化图表（ASCII）"""
    charts = []
    
    # 成功率对比
    success_rates = {group: stats["success_rate"] for group, stats in all_stats.items()}
    charts.append(generate_ascii_bar_chart(success_rates, "成功率对比"))
    
    # 平均工具调用对比
    tool_calls = {group: stats["avg_tool_calls"] for group, stats in all_stats.items()}
    charts.append(generate_ascii_bar_chart(tool_calls, "平均工具调用对比"))
    
    # 平均满意度对比
    satisfactions = {group: stats["avg_satisfaction"] for group, stats in all_stats.items()}
    charts.append(generate_ascii_bar_chart(satisfactions, "平均满意度对比"))
    
    # 保存图表
    output_file = VISUALIZATIONS_DIR / "ascii_charts.txt"
    with open(output_file, "w", encoding="utf-8") as f:
        f.write("".join(charts))
    print(f"ASCII 图表已保存到：{output_file}")


# ============================================================================
# 主函数
# ============================================================================

def main():
    print("="*60)
    print("实验结果分析")
    print("="*60)
    print()
    
    # 加载日志
    print("加载实验日志...")
    all_logs = load_all_logs()
    
    if not all_logs:
        print("❌ 未找到实验日志，请先运行基准测试")
        print(f"   运行：python {EXPERIMENTS_DIR / 'scripts' / 'run_benchmark.py'} --all-groups")
        return
    
    print(f"✓ 找到 {len(all_logs)} 个实验组的数据")
    for group, logs in all_logs.items():
        print(f"  - {group}: {len(logs)} 个任务")
    print()
    
    # 计算统计
    print("计算统计指标...")
    all_stats = {group: calculate_group_stats(logs) for group, logs in all_logs.items()}
    
    # 对比分析
    print("对比分析...")
    comparisons = compare_groups(all_stats)
    
    # 生成报告
    print("生成报告...")
    report = generate_comparison_report(all_stats, comparisons)
    save_report(report)
    
    # 保存 JSON
    save_json_stats(all_stats, comparisons)
    
    # 生成可视化
    print("生成可视化图表...")
    generate_visualizations(all_stats)
    
    # 打印摘要
    print("\n" + "="*60)
    print("分析完成！摘要:")
    print("="*60)
    
    for group, stats in all_stats.items():
        print(f"\n{group}:")
        print(f"  任务数：{stats['total_tasks']}")
        print(f"  成功率：{stats['success_rate']:.1%}")
        print(f"  平均工具调用：{stats['avg_tool_calls']:.1f}")
        print(f"  平均执行时间：{stats['avg_execution_time_ms']:.0f}ms")
        print(f"  平均满意度：{stats['avg_satisfaction']:.1f}/5")
    
    if comparisons:
        print("\n相对 Control 组的改进:")
        for group, improvement in comparisons.items():
            print(f"\n{group}:")
            print(f"  成功率：{improvement['success_rate_improvement']:+.1f}%")
            print(f"  工具调用：{improvement['tool_calls_reduction']:+.1f}%")
            print(f"  满意度：{improvement['satisfaction_improvement']:+.1f}%")
    
    print("\n" + "="*60)


if __name__ == "__main__":
    main()
