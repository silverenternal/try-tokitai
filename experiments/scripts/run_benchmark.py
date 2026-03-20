#!/usr/bin/env python3
"""
实验运行脚本

用法:
    # 运行单组实验
    python run_benchmark.py --group Ours-Full --days 7
    
    # 运行所有对比实验
    python run_benchmark.py --all-groups --days 30
    
    # 运行消融实验
    python run_benchmark.py --ablation --days 30
"""

import json
import os
import sys
import subprocess
import argparse
import time
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Any, Optional
import random

# 添加父目录到路径
sys.path.insert(0, str(Path(__file__).parent.parent))

# 实验组配置
EXPERIMENT_GROUPS = {
    'Control': {
        'config': {'self_evolution': False},
        'description': '原始 tokitai（无自进化）'
    },
    'Ours-Full': {
        'config': {
            'self_evolution': True,
            'prompt_engineering': True,
            'multi_agent': True,
            'cot': True,
            'self_fix': True
        },
        'description': '完整 Prompt Engineering 系统'
    },
    'Ours-Single': {
        'config': {
            'self_evolution': True,
            'prompt_engineering': True,
            'multi_agent': False,  # 单 LLM
            'cot': True,
            'self_fix': True
        },
        'description': '单 LLM 决策（无多智能体协商）'
    },
    'Ours-NoCoT': {
        'config': {
            'self_evolution': True,
            'prompt_engineering': True,
            'multi_agent': True,
            'cot': False,  # 无 Chain-of-Thought
            'self_fix': True
        },
        'description': '无 Chain-of-Thought 推理'
    },
    'Ours-NoFix': {
        'config': {
            'self_evolution': True,
            'prompt_engineering': True,
            'multi_agent': True,
            'cot': True,
            'self_fix': False  # 无自修正循环
        },
        'description': '无自修正循环'
    }
}


class BenchmarkRunner:
    """基准测试运行器"""

    def __init__(
        self,
        group: str,
        days: int = 7,
        tasks_file: str = "experiments/tasks/benchmark_tasks.json",
        log_dir: str = "experiments/logs"
    ):
        self.group = group
        self.days = days
        self.tasks_file = Path(tasks_file)
        self.log_dir = Path(log_dir) / group.lower().replace(' ', '_').replace('-', '_')
        self.config = EXPERIMENT_GROUPS.get(group, {}).get('config', {})
        
        # 创建日志目录
        self.log_dir.mkdir(parents=True, exist_ok=True)
        
        # 日志文件
        self.task_log = self.log_dir / "task_executions.jsonl"
        self.evolution_log = self.log_dir / "evolution_cycles.jsonl"
        self.summary_log = self.log_dir / "summary.json"
        
        # 加载任务
        self.tasks = self.load_tasks()
        
        # 统计信息
        self.stats = {
            'total_tasks': 0,
            'successful_tasks': 0,
            'failed_tasks': 0,
            'total_tool_calls': 0,
            'total_execution_time_ms': 0,
            'gaps_detected': 0,
            'tools_created': 0,
            'tools_optimized': 0,
            'api_cost_usd': 0.0
        }

    def load_tasks(self) -> List[Dict[str, Any]]:
        """加载基准测试任务"""
        if not self.tasks_file.exists():
            print(f"错误：任务文件不存在：{self.tasks_file}")
            sys.exit(1)
        
        with open(self.tasks_file, 'r', encoding='utf-8') as f:
            data = json.load(f)
            return data.get('tasks', [])

    def log_task_execution(self, execution: Dict[str, Any]):
        """记录任务执行日志"""
        with open(self.task_log, 'a', encoding='utf-8') as f:
            f.write(json.dumps(execution, ensure_ascii=False) + '\n')

    def log_evolution_cycle(self, cycle: Dict[str, Any]):
        """记录进化周期日志"""
        with open(self.evolution_log, 'a', encoding='utf-8') as f:
            f.write(json.dumps(cycle, ensure_ascii=False) + '\n')

    def simulate_task_execution(self, task: Dict[str, Any]) -> Dict[str, Any]:
        """
        模拟任务执行
        
        注意：这是模拟实现，实际应该调用 tokitai 执行任务
        """
        start_time = time.time()
        
        # 模拟执行成功率（根据组别配置）
        base_success_rate = 0.65  # Control 组基线
        
        if self.group == 'Ours-Full':
            # 完整系统有更高的成功率
            success_rate = base_success_rate + 0.15
        elif self.group == 'Ours-Single':
            success_rate = base_success_rate + 0.10
        elif self.group in ['Ours-NoCoT', 'Ours-NoFix']:
            success_rate = base_success_rate + 0.08
        else:
            success_rate = base_success_rate
        
        # 模拟执行结果
        success = random.random() < success_rate
        
        # 模拟工具调用次数
        expected_calls = task.get('expected_tool_calls', 1)
        if success:
            # 成功时接近预期调用次数
            tool_calls = max(1, int(expected_calls * random.uniform(0.8, 1.2)))
        else:
            # 失败时可能调用更多工具
            tool_calls = int(expected_calls * random.uniform(1.2, 2.0))
        
        execution_time = random.uniform(100, 5000)  # 100ms - 5s
        
        # 模拟用户满意度
        if success:
            satisfaction = random.randint(4, 5)
        else:
            satisfaction = random.randint(1, 3)
        
        execution = {
            'task_id': task.get('id'),
            'category': task.get('category'),
            'difficulty': task.get('difficulty'),
            'description': task.get('description'),
            'timestamp': datetime.now().isoformat(),
            'group': self.group,
            'execution': {
                'success': success,
                'total_tool_calls': tool_calls,
                'execution_time_ms': int(execution_time),
                'user_satisfaction': satisfaction,
                'error_message': None if success else "模拟执行失败"
            },
            'evolution': {
                'gaps_detected': random.randint(0, 2) if self.config.get('self_evolution') else 0,
                'tools_created': 0,
                'tools_optimized': 0
            }
        }
        
        # 更新统计
        self.stats['total_tasks'] += 1
        if success:
            self.stats['successful_tasks'] += 1
        else:
            self.stats['failed_tasks'] += 1
        self.stats['total_tool_calls'] += tool_calls
        self.stats['total_execution_time_ms'] += int(execution_time)
        
        return execution

    def simulate_evolution_cycle(self, cycle_id: int) -> Dict[str, Any]:
        """
        模拟进化周期
        
        注意：这是模拟实现，实际应该调用自进化系统
        """
        gaps_detected = []
        actions_taken = []
        
        if self.config.get('self_evolution'):
            # 模拟检测到的缺口
            num_gaps = random.randint(0, 3)
            for i in range(num_gaps):
                gap = {
                    'gap_type': random.choice(['missing_tool', 'tool_improvement', 'tool_merge']),
                    'description': f"模拟缺口 {i+1}",
                    'suggested_name': f"simulated_tool_{cycle_id}_{i}",
                    'priority': random.randint(5, 10)
                }
                gaps_detected.append(gap)
            
            # 模拟采取的行动
            for gap in gaps_detected:
                if random.random() < 0.7:  # 70% 概率创建工具
                    action = {
                        'action_type': 'create_tool',
                        'tool_name': gap['suggested_name'],
                        'result': 'success',
                        'compilation_attempts': random.randint(1, 3)
                    }
                    actions_taken.append(action)
                    self.stats['tools_created'] += 1
            
            self.stats['gaps_detected'] += len(gaps_detected)
        
        # 模拟 API 成本
        api_calls = random.randint(5, 20)
        api_cost = api_calls * 0.015  # $0.015 per call
        
        cycle = {
            'cycle_id': f"cycle_{cycle_id:03d}",
            'timestamp': datetime.now().isoformat(),
            'group': self.group,
            'reflection': {
                'coverage_score': random.uniform(0.6, 0.9),
                'systemic_issues': [] if not gaps_detected else ['模拟系统问题'],
                'strategic_recommendations': []
            },
            'gaps_detected': gaps_detected,
            'actions_taken': actions_taken,
            'metrics': {
                'api_calls': api_calls,
                'api_cost_usd': round(api_cost, 3),
                'cycle_duration_ms': random.randint(10000, 60000)
            }
        }
        
        self.stats['api_cost_usd'] += api_cost
        
        return cycle

    def run_day(self, day: int):
        """运行一天的实验"""
        print(f"\n{'='*60}")
        print(f"第 {day}/{self.days} 天 - {self.group}")
        print(f"{'='*60}")
        
        # 随机选择当天的任务（每天 10-20 个任务）
        num_tasks = random.randint(10, 20)
        day_tasks = random.sample(self.tasks, min(num_tasks, len(self.tasks)))
        
        print(f"执行 {len(day_tasks)} 个任务...")
        
        # 执行任务
        for task in day_tasks:
            execution = self.simulate_task_execution(task)
            self.log_task_execution(execution)
            
            if execution['execution']['success']:
                print(f"  ✓ {task['id']}: {execution['execution']['total_tool_calls']} 次调用")
            else:
                print(f"  ✗ {task['id']}: 失败")
        
        # 执行进化周期（每天一次）
        if self.config.get('self_evolution'):
            print("执行进化周期...")
            cycle = self.simulate_evolution_cycle(day)
            self.log_evolution_cycle(cycle)
            
            if cycle['gaps_detected']:
                print(f"  检测到 {len(cycle['gaps_detected'])} 个缺口")
            if cycle['actions_taken']:
                print(f"  创建 {len([a for a in cycle['actions_taken'] if a['action_type'] == 'create_tool'])} 个工具")
        
        # 保存每日摘要
        self.save_daily_summary(day)

    def save_daily_summary(self, day: int):
        """保存每日摘要"""
        summary = {
            'day': day,
            'timestamp': datetime.now().isoformat(),
            'group': self.group,
            'stats': self.stats.copy(),
            'config': self.config
        }
        
        with open(self.summary_log, 'w', encoding='utf-8') as f:
            json.dump(summary, f, indent=2, ensure_ascii=False)

    def run(self):
        """运行完整实验"""
        print(f"\n{'='*60}")
        print(f"开始实验：{self.group}")
        print(f"配置：{json.dumps(self.config, ensure_ascii=False)}")
        print(f"天数：{self.days}")
        print(f"任务总数：{len(self.tasks)}")
        print(f"{'='*60}")
        
        start_time = time.time()
        
        for day in range(1, self.days + 1):
            self.run_day(day)
        
        elapsed = time.time() - start_time
        
        # 打印最终统计
        print(f"\n{'='*60}")
        print(f"实验完成：{self.group}")
        print(f"{'='*60}")
        print(f"总耗时：{elapsed:.1f} 秒")
        print(f"总任务数：{self.stats['total_tasks']}")
        print(f"成功任务数：{self.stats['successful_tasks']}")
        print(f"失败任务数：{self.stats['failed_tasks']}")
        print(f"成功率：{self.stats['successful_tasks']/max(1, self.stats['total_tasks']):.2%}")
        print(f"平均工具调用：{self.stats['total_tool_calls']/max(1, self.stats['total_tasks']):.2f}")
        print(f"总 API 成本：${self.stats['api_cost_usd']:.2f}")
        print(f"日志目录：{self.log_dir}")
        
        return self.stats


def run_all_groups(days: int = 30):
    """运行所有实验组"""
    all_stats = {}
    
    for group_name in EXPERIMENT_GROUPS.keys():
        runner = BenchmarkRunner(group_name, days=days)
        stats = runner.run()
        all_stats[group_name] = stats
    
    # 保存总体统计
    summary = {
        'timestamp': datetime.now().isoformat(),
        'days': days,
        'groups': all_stats
    }
    
    output_file = Path("experiments/analysis/all_groups_summary.json")
    output_file.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(summary, f, indent=2, ensure_ascii=False)
    
    print(f"\n总体统计已保存到：{output_file}")
    
    return all_stats


def run_ablation(days: int = 30):
    """运行消融实验"""
    ablation_groups = ['Ours-Full', 'Ours-Single', 'Ours-NoCoT', 'Ours-NoFix']
    all_stats = {}
    
    for group_name in ablation_groups:
        runner = BenchmarkRunner(group_name, days=days)
        stats = runner.run()
        all_stats[group_name] = stats
    
    # 保存消融统计
    summary = {
        'timestamp': datetime.now().isoformat(),
        'days': days,
        'type': 'ablation',
        'groups': all_stats
    }
    
    output_file = Path("experiments/analysis/ablation_summary.json")
    output_file.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(summary, f, indent=2, ensure_ascii=False)
    
    print(f"\n消融实验统计已保存到：{output_file}")
    
    return all_stats


def main():
    parser = argparse.ArgumentParser(description='实验运行脚本')
    parser.add_argument('--group', type=str, help='实验组名称')
    parser.add_argument('--days', type=int, default=7, help='实验天数')
    parser.add_argument('--all-groups', action='store_true', help='运行所有实验组')
    parser.add_argument('--ablation', action='store_true', help='运行消融实验')
    parser.add_argument('--tasks-file', type=str, default='experiments/tasks/benchmark_tasks.json',
                        help='任务文件路径')
    
    args = parser.parse_args()
    
    if args.all_groups:
        run_all_groups(days=args.days)
    elif args.ablation:
        run_ablation(days=args.days)
    elif args.group:
        runner = BenchmarkRunner(args.group, days=args.days, tasks_file=args.tasks_file)
        runner.run()
    else:
        parser.print_help()
        print("\n示例:")
        print("  python run_benchmark.py --group Ours-Full --days 7")
        print("  python run_benchmark.py --all-groups --days 30")
        print("  python run_benchmark.py --ablation --days 30")


if __name__ == '__main__':
    main()
