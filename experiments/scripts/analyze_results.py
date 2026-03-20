#!/usr/bin/env python3
"""
实验结果分析脚本

用法:
    python analyze_results.py --group Ours-Full
    python analyze_results.py --compare Control Ours-Full Ours-Single
    python analyze_results.py --ablation
"""

import json
import os
import sys
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Any
import statistics

# 添加父目录到路径
sys.path.insert(0, str(Path(__file__).parent.parent))

class ResultAnalyzer:
    """实验结果分析器"""
    
    def __init__(self, logs_dir: str = "experiments/logs"):
        self.logs_dir = Path(logs_dir)
        
    def load_task_executions(self, group: str) -> List[Dict[str, Any]]:
        """加载任务执行日志"""
        log_file = self.logs_dir / group.lower() / "task_executions.jsonl"
        if not log_file.exists():
            print(f"警告：日志文件不存在：{log_file}")
            return []
        
        executions = []
        with open(log_file, 'r', encoding='utf-8') as f:
            for line in f:
                line = line.strip()
                if line:
                    try:
                        executions.append(json.loads(line))
                    except json.JSONDecodeError as e:
                        print(f"解析错误：{e}")
        
        return executions
    
    def load_evolution_cycles(self, group: str) -> List[Dict[str, Any]]:
        """加载进化周期日志"""
        log_file = self.logs_dir / group.lower() / "evolution_cycles.jsonl"
        if not log_file.exists():
            return []
        
        cycles = []
        with open(log_file, 'r', encoding='utf-8') as f:
            for line in f:
                line = line.strip()
                if line:
                    try:
                        cycles.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass
        
        return cycles
    
    def calculate_metrics(self, executions: List[Dict[str, Any]]) -> Dict[str, float]:
        """计算性能指标"""
        if not executions:
            return {}
        
        total = len(executions)
        successful = sum(1 for e in executions if e.get('execution', {}).get('success', False))
        success_rate = successful / total if total > 0 else 0.0
        
        tool_calls = [e.get('execution', {}).get('total_tool_calls', 0) for e in executions]
        avg_tool_calls = statistics.mean(tool_calls) if tool_calls else 0.0
        
        execution_times = [e.get('execution', {}).get('execution_time_ms', 0) for e in executions]
        avg_execution_time = statistics.mean(execution_times) if execution_times else 0.0
        
        satisfactions = [
            e.get('execution', {}).get('user_satisfaction', 0)
            for e in executions
            if e.get('execution', {}).get('user_satisfaction') is not None
        ]
        avg_satisfaction = statistics.mean(satisfactions) if satisfactions else 0.0
        
        return {
            'total_tasks': total,
            'successful_tasks': successful,
            'success_rate': success_rate,
            'avg_tool_calls': avg_tool_calls,
            'avg_execution_time_ms': avg_execution_time,
            'avg_satisfaction': avg_satisfaction,
        }
    
    def compare_groups(self, groups: List[str]) -> Dict[str, Dict[str, float]]:
        """比较多个实验组"""
        results = {}
        for group in groups:
            executions = self.load_task_executions(group)
            metrics = self.calculate_metrics(executions)
            results[group] = metrics
        
        return results
    
    def generate_report(self, output_file: str = "experiments/analysis/report.md"):
        """生成分析报告"""
        groups = ['Control', 'Ours-Full', 'Ours-Single', 'Ours-NoCoT', 'Ours-NoFix']
        results = self.compare_groups(groups)
        
        output_path = Path(output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write("# 实验结果分析报告\n\n")
            f.write(f"**生成时间**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
            
            # 总体对比
            f.write("## 总体性能对比\n\n")
            f.write("| 实验组 | 总任务数 | 成功率 | 平均工具调用 | 平均执行时间 (ms) | 平均满意度 |\n")
            f.write("|--------|----------|--------|--------------|-------------------|------------|\n")
            
            for group in groups:
                if group not in results:
                    continue
                metrics = results[group]
                f.write(f"| {group} | ")
                f.write(f"{metrics.get('total_tasks', 0)} | ")
                f.write(f"{metrics.get('success_rate', 0):.2%} | ")
                f.write(f"{metrics.get('avg_tool_calls', 0):.2f} | ")
                f.write(f"{metrics.get('avg_execution_time_ms', 0):.0f} | ")
                f.write(f"{metrics.get('avg_satisfaction', 0):.2f} |\n")
            
            f.write("\n")
            
            # 关键发现
            f.write("## 关键发现\n\n")
            
            if 'Ours-Full' in results and 'Control' in results:
                control = results['Control']
                ours = results['Ours-Full']
                
                success_improvement = (
                    (ours.get('success_rate', 0) - control.get('success_rate', 0)) / 
                    control.get('success_rate', 1) * 100
                ) if control.get('success_rate', 0) > 0 else 0
                
                tool_calls_reduction = (
                    (control.get('avg_tool_calls', 0) - ours.get('avg_tool_calls', 0)) / 
                    control.get('avg_tool_calls', 1) * 100
                ) if control.get('avg_tool_calls', 0) > 0 else 0
                
                f.write(f"### 1. 任务完成率提升\n\n")
                f.write(f"- Control 组成功率：{control.get('success_rate', 0):.2%}\n")
                f.write(f"- Ours-Full 组成功率：{ours.get('success_rate', 0):.2%}\n")
                f.write(f"- **相对提升**: {success_improvement:.1f}%\n\n")
                
                f.write(f"### 2. 工具调用效率\n\n")
                f.write(f"- Control 组平均工具调用：{control.get('avg_tool_calls', 0):.2f}次\n")
                f.write(f"- Ours-Full 组平均工具调用：{ours.get('avg_tool_calls', 0):.2f}次\n")
                f.write(f"- **减少**: {tool_calls_reduction:.1f}%\n\n")
            
            # 消融实验分析
            f.write("## 消融实验分析\n\n")
            
            if 'Ours-Full' in results and 'Ours-Single' in results:
                f.write("### 多智能体协商的价值\n\n")
                full = results['Ours-Full']
                single = results['Ours-Single']
                
                improvement = (
                    (full.get('success_rate', 0) - single.get('success_rate', 0)) / 
                    single.get('success_rate', 1) * 100
                ) if single.get('success_rate', 0) > 0 else 0
                
                f.write(f"- Ours-Full (多智能体): {full.get('success_rate', 0):.2%}\n")
                f.write(f"- Ours-Single (单 LLM): {single.get('success_rate', 0):.2%}\n")
                f.write(f"- **提升**: {improvement:.1f}%\n\n")
            
            if 'Ours-Full' in results and 'Ours-NoCoT' in results:
                f.write("### Chain-of-Thought 的价值\n\n")
                full = results['Ours-Full']
                nocot = results['Ours-NoCoT']
                
                improvement = (
                    (full.get('success_rate', 0) - nocot.get('success_rate', 0)) / 
                    nocot.get('success_rate', 1) * 100
                ) if nocot.get('success_rate', 0) > 0 else 0
                
                f.write(f"- Ours-Full (有 CoT): {full.get('success_rate', 0):.2%}\n")
                f.write(f"- Ours-NoCoT (无 CoT): {nocot.get('success_rate', 0):.2%}\n")
                f.write(f"- **提升**: {improvement:.1f}%\n\n")
            
            if 'Ours-Full' in results and 'Ours-NoFix' in results:
                f.write("### 自修正循环的价值\n\n")
                full = results['Ours-Full']
                nofix = results['Ours-NoFix']
                
                improvement = (
                    (full.get('success_rate', 0) - nofix.get('success_rate', 0)) / 
                    nofix.get('success_rate', 1) * 100
                ) if nofix.get('success_rate', 0) > 0 else 0
                
                f.write(f"- Ours-Full (有自修正): {full.get('success_rate', 0):.2%}\n")
                f.write(f"- Ours-NoFix (无自修正): {nofix.get('success_rate', 0):.2%}\n")
                f.write(f"- **提升**: {improvement:.1f}%\n\n")
            
            # 进化周期分析
            f.write("## 进化周期分析\n\n")
            
            for group in groups:
                cycles = self.load_evolution_cycles(group)
                if not cycles:
                    continue
                
                total_gaps = sum(len(c.get('gaps_detected', [])) for c in cycles)
                total_created = sum(
                    len([a for a in c.get('actions_taken', []) if a.get('action_type') == 'create_tool'])
                    for c in cycles
                )
                
                f.write(f"### {group}\n\n")
                f.write(f"- 总周期数：{len(cycles)}\n")
                f.write(f"- 总缺口检测：{total_gaps}\n")
                f.write(f"- 总工具创建：{total_created}\n")
                f.write(f"- 平均每周期创建：{total_created/len(cycles):.2f}\n\n")
        
        print(f"报告已生成：{output_file}")
        return output_file


def main():
    import argparse
    
    parser = argparse.ArgumentParser(description='实验结果分析')
    parser.add_argument('--compare', nargs='+', 
                        default=['Control', 'Ours-Full', 'Ours-Single', 'Ours-NoCoT', 'Ours-NoFix'],
                        help='要比较的实验组')
    parser.add_argument('--output', default='experiments/analysis/report.md',
                        help='输出报告路径')
    parser.add_argument('--logs-dir', default='experiments/logs',
                        help='日志目录')
    
    args = parser.parse_args()
    
    analyzer = ResultAnalyzer(args.logs_dir)
    analyzer.generate_report(args.output)


if __name__ == '__main__':
    main()
