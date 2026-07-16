#!/usr/bin/env python3
"""
可视化图表生成脚本

用法:
    python generate_charts.py --input experiments/analysis/all_groups_summary.json
    python generate_charts.py --ablation
"""

import json
import os
import sys
import argparse
from pathlib import Path
from typing import Dict, List, Any, Optional
from datetime import datetime

# 检查 matplotlib 是否安装
try:
    import matplotlib.pyplot as plt
    import matplotlib
    matplotlib.use('Agg')  # 非交互式后端
    HAS_MATPLOTLIB = True
except ImportError:
    HAS_MATPLOTLIB = False
    print("警告：matplotlib 未安装，将生成文本格式的图表数据")
    print("安装：pip install matplotlib")

# 检查 seaborn 是否安装
try:
    import seaborn as sns
    HAS_SEABORN = True
except ImportError:
    HAS_SEABORN = False


class ChartGenerator:
    """图表生成器"""

    def __init__(self, output_dir: str = "experiments/analysis/visualizations"):
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)
        
        # 设置图表样式
        if HAS_SEABORN:
            sns.set_theme(style="whitegrid")
            sns.set_palette("husl")

    def load_data(self, json_file: str) -> Dict[str, Any]:
        """加载实验数据"""
        with open(json_file, 'r', encoding='utf-8') as f:
            return json.load(f)

    def generate_bar_chart(self, data: Dict[str, float], title: str, 
                          xlabel: str, ylabel: str, filename: str,
                          color: str = '#3498db'):
        """生成柱状图"""
        if not HAS_MATPLOTLIB:
            self._save_text_chart(data, title, filename)
            return
        
        fig, ax = plt.subplots(figsize=(10, 6))
        
        groups = list(data.keys())
        values = list(data.values())
        
        bars = ax.bar(groups, values, color=color, alpha=0.8)
        
        # 添加数值标签
        for bar, value in zip(bars, values):
            height = bar.get_height()
            ax.text(bar.get_x() + bar.get_width()/2., height,
                   f'{value:.2f}',
                   ha='center', va='bottom', fontsize=10)
        
        ax.set_title(title, fontsize=14, fontweight='bold')
        ax.set_xlabel(xlabel, fontsize=12)
        ax.set_ylabel(ylabel, fontsize=12)
        ax.tick_params(axis='x', rotation=45)
        
        plt.tight_layout()
        
        output_path = self.output_dir / filename
        plt.savefig(output_path, dpi=300, bbox_inches='tight')
        plt.close()
        
        print(f"已生成图表：{output_path}")

    def generate_grouped_bar_chart(self, data: Dict[str, Dict[str, float]], 
                                   title: str, xlabel: str, ylabel: str, 
                                   filename: str):
        """生成分组柱状图"""
        if not HAS_MATPLOTLIB:
            self._save_text_chart(data, title, filename)
            return
        
        fig, ax = plt.subplots(figsize=(12, 7))
        
        groups = list(data.keys())
        metrics = list(list(data.values())[0].keys())
        
        x = range(len(groups))
        width = 0.8 / len(metrics)
        
        colors = ['#3498db', '#e74c3c', '#2ecc71', '#f39c12', '#9b59b6']
        
        for i, metric in enumerate(metrics):
            values = [data[group].get(metric, 0) for group in groups]
            offset = [j + (i - len(metrics)/2 + 0.5) * width for j in x]
            
            bars = ax.bar(offset, values, width, 
                         label=metric, color=colors[i % len(colors)], alpha=0.8)
            
            # 添加数值标签
            for bar, value in zip(bars, values):
                height = bar.get_height()
                if height > 0:  # 只显示正值
                    ax.text(bar.get_x() + bar.get_width()/2., height,
                           f'{value:.2f}',
                           ha='center', va='bottom', fontsize=8)
        
        ax.set_title(title, fontsize=14, fontweight='bold')
        ax.set_xlabel(xlabel, fontsize=12)
        ax.set_ylabel(ylabel, fontsize=12)
        ax.set_xticks([i + 0.4 for i in x])
        ax.set_xticklabels(groups, rotation=45)
        ax.legend()
        
        plt.tight_layout()
        
        output_path = self.output_dir / filename
        plt.savefig(output_path, dpi=300, bbox_inches='tight')
        plt.close()
        
        print(f"已生成图表：{output_path}")

    def generate_line_chart(self, data: List[Dict[str, Any]], x_key: str,
                           y_key: str, title: str, xlabel: str, ylabel: str,
                           filename: str, color: str = '#3498db'):
        """生成折线图"""
        if not HAS_MATPLOTLIB:
            self._save_text_chart(data, title, filename)
            return
        
        fig, ax = plt.subplots(figsize=(12, 7))
        
        x_values = [d[x_key] for d in data]
        y_values = [d[y_key] for d in data]
        
        ax.plot(x_values, y_values, marker='o', linewidth=2, 
               markersize=6, color=color, label=y_key)
        
        ax.set_title(title, fontsize=14, fontweight='bold')
        ax.set_xlabel(xlabel, fontsize=12)
        ax.set_ylabel(ylabel, fontsize=12)
        ax.legend()
        ax.grid(True, alpha=0.3)
        
        plt.tight_layout()
        
        output_path = self.output_dir / filename
        plt.savefig(output_path, dpi=300, bbox_inches='tight')
        plt.close()
        
        print(f"已生成图表：{output_path}")

    def generate_learning_curve(self, daily_stats: List[Dict[str, Any]], 
                               groups: List[str], filename: str):
        """生成学习曲线（多组对比）"""
        if not HAS_MATPLOTLIB:
            self._save_text_chart(daily_stats, "Learning Curve", filename)
            return
        
        fig, ax = plt.subplots(figsize=(14, 8))
        
        colors = ['#3498db', '#e74c3c', '#2ecc71', '#f39c12', '#9b59b6']
        
        for i, group in enumerate(groups):
            group_data = [d for d in daily_stats if d.get('group') == group]
            if not group_data:
                continue
            
            days = [d.get('day', 0) for d in group_data]
            success_rates = [
                d.get('stats', {}).get('successful_tasks', 0) / 
                max(1, d.get('stats', {}).get('total_tasks', 1))
                for d in group_data
            ]
            
            ax.plot(days, success_rates, marker='o', linewidth=2,
                   markersize=6, color=colors[i % len(colors)],
                   label=group)
        
        ax.set_title('Learning Curve: Task Success Rate Over Time', 
                    fontsize=14, fontweight='bold')
        ax.set_xlabel('Day', fontsize=12)
        ax.set_ylabel('Success Rate', fontsize=12)
        ax.legend()
        ax.grid(True, alpha=0.3)
        
        plt.tight_layout()
        
        output_path = self.output_dir / filename
        plt.savefig(output_path, dpi=300, bbox_inches='tight')
        plt.close()
        
        print(f"已生成图表：{output_path}")

    def _save_text_chart(self, data: Any, title: str, filename: str):
        """保存文本格式的图表数据（当 matplotlib 不可用时）"""
        output_path = self.output_dir / filename.replace('.png', '.txt')
        
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write(f"# {title}\n\n")
            f.write(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
            f.write("数据:\n")
            f.write(json.dumps(data, indent=2, ensure_ascii=False))
        
        print(f"已保存文本数据：{output_path}")

    def generate_all_charts(self, summary_file: str):
        """生成所有图表"""
        if not Path(summary_file).exists():
            print(f"错误：文件不存在：{summary_file}")
            return
        
        data = self.load_data(summary_file)
        groups = data.get('groups', {})
        
        print(f"\n{'='*60}")
        print("生成对比图表...")
        print(f"{'='*60}")
        
        # 1. 成功率对比
        success_rates = {}
        for group, stats in groups.items():
            total = stats.get('total_tasks', 1)
            successful = stats.get('successful_tasks', 0)
            success_rates[group] = successful / max(1, total) * 100
        
        self.generate_bar_chart(
            success_rates,
            'Task Success Rate by Group',
            'Experiment Group',
            'Success Rate (%)',
            'success_rate_comparison.png',
            color='#3498db'
        )
        
        # 2. 平均工具调用对比
        avg_tool_calls = {}
        for group, stats in groups.items():
            total = stats.get('total_tasks', 1)
            calls = stats.get('total_tool_calls', 0)
            avg_tool_calls[group] = calls / max(1, total)
        
        self.generate_bar_chart(
            avg_tool_calls,
            'Average Tool Calls per Task',
            'Experiment Group',
            'Tool Calls',
            'avg_tool_calls_comparison.png',
            color='#e74c3c'
        )
        
        # 3. 综合指标对比
        metrics_data = {}
        for group, stats in groups.items():
            total = max(1, stats.get('total_tasks', 1))
            metrics_data[group] = {
                'Success Rate (%)': stats.get('successful_tasks', 0) / total * 100,
                'Avg Tool Calls': stats.get('total_tool_calls', 0) / total,
                'API Cost ($)': stats.get('api_cost_usd', 0),
                'Tools Created': stats.get('tools_created', 0)
            }
        
        self.generate_grouped_bar_chart(
            metrics_data,
            'Multi-Metric Comparison Across Groups',
            'Experiment Group',
            'Value',
            'multi_metric_comparison.png'
        )
        
        # 4. API 成本对比
        api_costs = {group: stats.get('api_cost_usd', 0) 
                    for group, stats in groups.items()}
        
        self.generate_bar_chart(
            api_costs,
            'Total API Cost by Group',
            'Experiment Group',
            'Cost (USD)',
            'api_cost_comparison.png',
            color='#2ecc71'
        )
        
        print(f"\n所有图表已保存到：{self.output_dir}")

    def generate_ablation_charts(self, ablation_file: str):
        """生成消融实验图表"""
        if not Path(ablation_file).exists():
            print(f"错误：文件不存在：{ablation_file}")
            return
        
        data = self.load_data(ablation_file)
        groups = data.get('groups', {})
        
        print(f"\n{'='*60}")
        print("生成消融实验图表...")
        print(f"{'='*60}")
        
        # 1. 成功率对比（消融）
        success_rates = {}
        for group, stats in groups.items():
            total = stats.get('total_tasks', 1)
            successful = stats.get('successful_tasks', 0)
            success_rates[group] = successful / max(1, total) * 100
        
        # 计算提升百分比
        full_rate = success_rates.get('Ours-Full', 0)
        ablation_improvements = {}
        for group, rate in success_rates.items():
            if group == 'Ours-Full':
                continue
            improvement = (full_rate - rate) / max(0.01, rate) * 100
            ablation_improvements[group] = improvement
        
        self.generate_bar_chart(
            success_rates,
            'Ablation Study: Success Rate Comparison',
            'Configuration',
            'Success Rate (%)',
            'ablation_success_rate.png',
            color='#9b59b6'
        )
        
        # 2. 组件价值（提升百分比）
        component_names = {
            'Ours-Single': 'w/o Multi-Agent',
            'Ours-NoCoT': 'w/o Chain-of-Thought',
            'Ours-NoFix': 'w/o Self-Correction'
        }
        
        renamed_improvements = {}
        for group, improvement in ablation_improvements.items():
            if group in component_names:
                renamed_improvements[component_names[group]] = improvement
        
        self.generate_bar_chart(
            renamed_improvements,
            'Component Contribution to Performance',
            'Removed Component',
            'Performance Drop (%)',
            'component_contribution.png',
            color='#f39c12'
        )
        
        print(f"\n消融实验图表已保存到：{self.output_dir}")


def main():
    parser = argparse.ArgumentParser(description='可视化图表生成脚本')
    parser.add_argument('--input', type=str, 
                       default='experiments/analysis/all_groups_summary.json',
                       help='输入 JSON 文件路径')
    parser.add_argument('--ablation', type=str,
                       default='experiments/analysis/ablation_summary.json',
                       help='消融实验 JSON 文件路径')
    parser.add_argument('--output-dir', type=str,
                       default='experiments/analysis/visualizations',
                       help='输出目录')
    parser.add_argument('--all', action='store_true',
                       help='生成所有图表（包括消融）')
    
    args = parser.parse_args()
    
    generator = ChartGenerator(args.output_dir)
    
    # 生成主对比图表
    if Path(args.input).exists():
        generator.generate_all_charts(args.input)
    else:
        print(f"警告：主对比文件不存在：{args.input}")
        print("请先运行：python run_benchmark.py --all-groups")
    
    # 生成消融图表
    if args.all or Path(args.ablation).exists():
        if Path(args.ablation).exists():
            generator.generate_ablation_charts(args.ablation)
        else:
            print(f"警告：消融实验文件不存在：{args.ablation}")
            print("请先运行：python run_benchmark.py --ablation")
    
    print(f"\n{'='*60}")
    print("图表生成完成！")
    print(f"{'='*60}")
    print(f"输出目录：{generator.output_dir}")
    
    if not HAS_MATPLOTLIB:
        print("\n注意：matplotlib 未安装，已保存为文本格式数据")
        print("安装后可生成 PNG 图表：pip install matplotlib seaborn")


if __name__ == '__main__':
    main()
