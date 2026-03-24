#!/usr/bin/env python3
"""
Tokitai 基准测试运行脚本

用于运行 Prompt Engineering 自进化系统的对比实验和消融实验

使用方法:
    # 运行单组基准测试
    python run_benchmark.py --group Ours-Full --days 30
    
    # 运行所有对比实验
    python run_benchmark.py --all-groups
    
    # 运行消融实验
    python run_benchmark.py --ablation

实验组别:
    - Control: 原始 tokitai（无自进化）
    - Ours-Full: 完整 Prompt Engineering 系统
    - Ours-Single: 单 LLM 决策（无多智能体协商）
    - Ours-NoCoT: 无 Chain-of-Thought 推理
    - Ours-NoFix: 无自修正循环
"""

import argparse
import json
import os
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Any


# ============================================================================
# 配置
# ============================================================================

EXPERIMENTS_DIR = Path(__file__).parent
TASKS_DIR = EXPERIMENTS_DIR / "tasks"
LOGS_DIR = EXPERIMENTS_DIR / "logs"
ANALYSIS_DIR = EXPERIMENTS_DIR / "analysis"

# 实验组别配置
GROUPS = {
    "Control": {
        "description": "原始 tokitai（无自进化）",
        "args": ["--no-autonomous"]
    },
    "Ours-Full": {
        "description": "完整 Prompt Engineering 系统",
        "args": ["--autonomous"]
    },
    "Ours-Single": {
        "description": "单 LLM 决策（无多智能体协商）",
        "args": ["--autonomous", "--single-agent"]
    },
    "Ours-NoCoT": {
        "description": "无 Chain-of-Thought 推理",
        "args": ["--autonomous", "--no-cot"]
    },
    "Ours-NoFix": {
        "description": "无自修正循环",
        "args": ["--autonomous", "--no-self-fix"]
    }
}


# ============================================================================
# 数据类
# ============================================================================

class TaskExecutionLog:
    """任务执行日志"""
    
    def __init__(
        self,
        task_id: str,
        category: str,
        difficulty: str,
        description: str,
        group: str
    ):
        self.task_id = task_id
        self.category = category
        self.difficulty = difficulty
        self.description = description
        self.group = group
        self.timestamp = datetime.utcnow().isoformat() + "Z"
        self.success = False
        self.tool_calls: List[Dict[str, Any]] = []
        self.total_tool_calls = 0
        self.execution_time_ms = 0
        self.user_satisfaction = 0
        self.gaps_detected = 0
        self.tools_created = 0
        self.tools_optimized = 0
        self.error_message: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """转换为字典格式"""
        return {
            "task_id": self.task_id,
            "category": self.category,
            "difficulty": self.difficulty,
            "description": self.description,
            "timestamp": self.timestamp,
            "group": self.group,
            "execution": {
                "success": self.success,
                "tool_calls": self.tool_calls,
                "total_tool_calls": self.total_tool_calls,
                "execution_time_ms": self.execution_time_ms,
                "user_satisfaction": self.user_satisfaction,
                "error_message": self.error_message
            },
            "evolution": {
                "gaps_detected": self.gaps_detected,
                "tools_created": self.tools_created,
                "tools_optimized": self.tools_optimized
            }
        }


class SelfEvolutionLog:
    """自进化日志"""
    
    def __init__(self, cycle_id: str, group: str):
        self.cycle_id = cycle_id
        self.group = group
        self.timestamp = datetime.utcnow().isoformat() + "Z"
        self.reflection: Dict[str, Any] = {}
        self.gaps_detected: List[Dict[str, Any]] = []
        self.actions_taken: List[Dict[str, Any]] = []
        self.metrics: Dict[str, Any] = {
            "api_calls": 0,
            "api_cost_usd": 0.0,
            "cycle_duration_ms": 0
        }
    
    def to_dict(self) -> Dict[str, Any]:
        """转换为字典格式"""
        return {
            "cycle_id": self.cycle_id,
            "timestamp": self.timestamp,
            "group": self.group,
            "reflection": self.reflection,
            "gaps_detected": self.gaps_detected,
            "actions_taken": self.actions_taken,
            "metrics": self.metrics
        }


# ============================================================================
# 基准测试运行器
# ============================================================================

class BenchmarkRunner:
    """基准测试运行器"""
    
    def __init__(self, group: str, days: int = 1, project_path: Optional[Path] = None):
        self.group = group
        self.days = days
        self.project_path = project_path or Path.cwd()
        self.logs: List[Dict[str, Any]] = []
        self.evolution_logs: List[Dict[str, Any]] = []
        
        # 创建日志目录
        self.log_dir = LOGS_DIR / group.lower().replace(" ", "_")
        self.log_dir.mkdir(parents=True, exist_ok=True)
    
    def load_tasks(self) -> List[Dict[str, Any]]:
        """加载基准测试任务"""
        tasks_file = TASKS_DIR / "benchmark_tasks.json"
        if not tasks_file.exists():
            raise FileNotFoundError(f"任务文件不存在：{tasks_file}")
        
        with open(tasks_file, "r", encoding="utf-8") as f:
            data = json.load(f)
        
        return data.get("tasks", [])
    
    def run_task(self, task: Dict[str, Any]) -> TaskExecutionLog:
        """运行单个任务"""
        log = TaskExecutionLog(
            task_id=task["id"],
            category=task["category"],
            difficulty=task["difficulty"],
            description=task["description"],
            group=self.group
        )
        
        start_time = time.time()
        
        try:
            # 构建命令
            cmd = [
                "cargo", "run", "--release", "--"
            ] + GROUPS[self.group]["args"] + [
                "--project-path", str(self.project_path),
                "--task", task["description"]
            ]
            
            # 执行任务（模拟）
            # 实际实现需要与 ai-assistant 交互
            print(f"  执行任务：{task['id']} - {task['description']}")
            
            # TODO: 实际实现需要：
            # 1. 启动 ai-assistant 进程
            # 2. 发送任务描述
            # 3. 捕获工具调用
            # 4. 等待任务完成
            # 5. 记录执行结果
            
            # 模拟执行（用于测试框架）
            time.sleep(0.1)  # 模拟执行时间
            log.success = True
            log.total_tool_calls = 2
            log.execution_time_ms = int((time.time() - start_time) * 1000)
            log.user_satisfaction = 4
            
        except Exception as e:
            log.success = False
            log.error_message = str(e)
            log.execution_time_ms = int((time.time() - start_time) * 1000)
        
        return log
    
    def run_evolution_cycle(self, cycle_num: int) -> SelfEvolutionLog:
        """运行一次自进化循环"""
        log = SelfEvolutionLog(cycle_id=f"cycle_{cycle_num:03d}", group=self.group)
        
        start_time = time.time()
        
        try:
            # TODO: 实际实现需要：
            # 1. 调用 HybridGapDetector 检测工具缺口
            # 2. 调用 PromptOptimizer 优化工具
            # 3. 调用 PromptCreator 创建新工具
            # 4. 调用 MultiAgentNegotiator 协商决策
            # 5. 记录执行结果
            
            # 模拟自进化（用于测试框架）
            time.sleep(0.5)  # 模拟执行时间
            
            log.reflection = {
                "coverage_score": 0.75,
                "systemic_issues": ["缺少批量文件处理工具"],
                "strategic_recommendations": ["优先发展文件批处理工具"]
            }
            
            log.gaps_detected = [
                {
                    "gap_type": "missing_tool",
                    "description": "缺少批量重命名文件的工具",
                    "suggested_name": "batch_rename_files",
                    "priority": 8
                }
            ]
            
            log.actions_taken = [
                {
                    "action_type": "create_tool",
                    "tool_name": "batch_rename_files",
                    "result": "success",
                    "compilation_attempts": 2
                }
            ]
            
            log.metrics = {
                "api_calls": 15,
                "api_cost_usd": 0.25,
                "cycle_duration_ms": int((time.time() - start_time) * 1000)
            }
            
        except Exception as e:
            log.metrics["error"] = str(e)
        
        return log
    
    def run_benchmark(self) -> Dict[str, Any]:
        """运行完整基准测试"""
        print(f"\n{'='*60}")
        print(f"基准测试：{self.group}")
        print(f"天数：{self.days}")
        print(f"项目路径：{self.project_path}")
        print(f"{'='*60}\n")
        
        tasks = self.load_tasks()
        print(f"加载任务：{len(tasks)} 个\n")
        
        # 运行任务
        for i, task in enumerate(tasks, 1):
            print(f"[{i}/{len(tasks)}] ", end="")
            log = self.run_task(task)
            self.logs.append(log.to_dict())
            
            # 每 5 个任务运行一次自进化循环
            if i % 5 == 0 and self.group != "Control":
                cycle_num = i // 5
                evo_log = self.run_evolution_cycle(cycle_num)
                self.evolution_logs.append(evo_log.to_dict())
        
        # 保存日志
        self.save_logs()
        
        # 生成摘要
        summary = self.generate_summary()
        
        print(f"\n基准测试完成！")
        print(f"  任务完成数：{summary['tasks_completed']}")
        print(f"  任务成功率：{summary['success_rate']:.1%}")
        print(f"  平均工具调用：{summary['avg_tool_calls']:.1f}")
        print(f"  平均执行时间：{summary['avg_execution_time_ms']}ms")
        print(f"  平均满意度：{summary['avg_satisfaction']:.1f}/5")
        
        if self.group != "Control":
            print(f"  检测缺口：{summary['gaps_detected']}")
            print(f"  创建工具：{summary['tools_created']}")
            print(f"  API 成本：${summary['api_cost_usd']:.2f}")
        
        return summary
    
    def save_logs(self):
        """保存日志到文件"""
        # 保存任务执行日志
        task_log_file = self.log_dir / f"task_logs_{datetime.now().strftime('%Y%m%d_%H%M%S')}.jsonl"
        with open(task_log_file, "w", encoding="utf-8") as f:
            for log in self.logs:
                f.write(json.dumps(log, ensure_ascii=False) + "\n")
        
        # 保存自进化日志
        if self.evolution_logs:
            evo_log_file = self.log_dir / f"evolution_logs_{datetime.now().strftime('%Y%m%d_%H%M%S')}.jsonl"
            with open(evo_log_file, "w", encoding="utf-8") as f:
                for log in self.evolution_logs:
                    f.write(json.dumps(log, ensure_ascii=False) + "\n")
        
        print(f"日志已保存到：{self.log_dir}")
    
    def generate_summary(self) -> Dict[str, Any]:
        """生成摘要统计"""
        if not self.logs:
            return {}
        
        # 任务执行统计
        completed = sum(1 for log in self.logs if log["execution"]["success"])
        total_tool_calls = sum(log["execution"]["total_tool_calls"] for log in self.logs)
        total_time = sum(log["execution"]["execution_time_ms"] for log in self.logs)
        total_satisfaction = sum(log["execution"]["user_satisfaction"] for log in self.logs)
        
        summary = {
            "group": self.group,
            "days": self.days,
            "tasks_completed": len(self.logs),
            "tasks_successful": completed,
            "success_rate": completed / len(self.logs) if self.logs else 0,
            "avg_tool_calls": total_tool_calls / len(self.logs) if self.logs else 0,
            "avg_execution_time_ms": total_time // len(self.logs) if self.logs else 0,
            "avg_satisfaction": total_satisfaction / len(self.logs) if self.logs else 0,
        }
        
        # 自进化统计
        if self.evolution_logs:
            total_gaps = sum(len(log["gaps_detected"]) for log in self.evolution_logs)
            total_created = sum(
                sum(1 for a in log["actions_taken"] if a["action_type"] == "create_tool")
                for log in self.evolution_logs
            )
            total_api_cost = sum(log["metrics"]["api_cost_usd"] for log in self.evolution_logs)
            
            summary["gaps_detected"] = total_gaps
            summary["tools_created"] = total_created
            summary["api_cost_usd"] = total_api_cost
        
        return summary


# ============================================================================
# 分析器
# ============================================================================

class ResultsAnalyzer:
    """结果分析器"""
    
    def __init__(self):
        self.results_dir = ANALYSIS_DIR
        self.results_dir.mkdir(parents=True, exist_ok=True)
    
    def load_group_logs(self, group: str) -> List[Dict[str, Any]]:
        """加载指定组的日志"""
        log_dir = LOGS_DIR / group.lower().replace(" ", "_")
        if not log_dir.exists():
            return []
        
        logs = []
        for log_file in log_dir.glob("task_logs_*.jsonl"):
            with open(log_file, "r", encoding="utf-8") as f:
                for line in f:
                    logs.append(json.loads(line))
        
        return logs
    
    def compare_groups(self, groups: List[str]) -> Dict[str, Any]:
        """对比多组结果"""
        results = {}
        
        for group in groups:
            logs = self.load_group_logs(group)
            if not logs:
                continue
            
            completed = sum(1 for log in logs if log["execution"]["success"])
            results[group] = {
                "tasks_completed": len(logs),
                "tasks_successful": completed,
                "success_rate": completed / len(logs) if logs else 0,
                "avg_tool_calls": sum(log["execution"]["total_tool_calls"] for log in logs) / len(logs) if logs else 0,
                "avg_execution_time_ms": sum(log["execution"]["execution_time_ms"] for log in logs) // len(logs) if logs else 0,
                "avg_satisfaction": sum(log["execution"]["user_satisfaction"] for log in logs) / len(logs) if logs else 0,
            }
        
        # 保存对比结果
        output_file = self.results_dir / "comparison_results.json"
        with open(output_file, "w", encoding="utf-8") as f:
            json.dump(results, f, indent=2, ensure_ascii=False)
        
        print(f"\n对比结果已保存到：{output_file}")
        
        return results
    
    def print_comparison(self, results: Dict[str, Any]):
        """打印对比表格"""
        print("\n" + "="*80)
        print("实验结果对比")
        print("="*80)
        
        # 表头
        print(f"{'组别':<15} {'任务数':>8} {'成功率':>10} {'平均工具':>10} {'平均时间':>12} {'满意度':>8}")
        print("-"*80)
        
        # 数据行
        for group, stats in results.items():
            print(
                f"{group:<15} "
                f"{stats['tasks_completed']:>8} "
                f"{stats['success_rate']:>10.1%} "
                f"{stats['avg_tool_calls']:>10.1f} "
                f"{stats['avg_execution_time_ms']:>12}ms "
                f"{stats['avg_satisfaction']:>8.1f}"
            )
        
        print("="*80)


# ============================================================================
# 主函数
# ============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Tokitai 基准测试运行脚本",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )
    
    parser.add_argument(
        "--group", "-g",
        choices=list(GROUPS.keys()),
        help="实验组别"
    )
    
    parser.add_argument(
        "--days", "-d",
        type=int,
        default=1,
        help="实验天数（默认：1）"
    )
    
    parser.add_argument(
        "--project-path", "-p",
        type=Path,
        help="项目路径（默认：当前目录）"
    )
    
    parser.add_argument(
        "--all-groups", "-a",
        action="store_true",
        help="运行所有对比实验组"
    )
    
    parser.add_argument(
        "--ablation",
        action="store_true",
        help="运行消融实验"
    )
    
    parser.add_argument(
        "--analyze",
        action="store_true",
        help="分析已有实验结果"
    )
    
    args = parser.parse_args()
    
    # 分析模式
    if args.analyze:
        analyzer = ResultsAnalyzer()
        groups = list(GROUPS.keys())
        results = analyzer.compare_groups(groups)
        analyzer.print_comparison(results)
        return
    
    # 运行所有组
    if args.all_groups:
        all_results = {}
        for group in GROUPS.keys():
            runner = BenchmarkRunner(group, args.days, args.project_path)
            result = runner.run_benchmark()
            all_results[group] = result
        
        # 对比分析
        analyzer = ResultsAnalyzer()
        analyzer.print_comparison(all_results)
        return
    
    # 运行消融实验
    if args.ablation:
        ablation_groups = ["Ours-Full", "Ours-Single", "Ours-NoCoT", "Ours-NoFix"]
        all_results = {}
        for group in ablation_groups:
            runner = BenchmarkRunner(group, args.days, args.project_path)
            result = runner.run_benchmark()
            all_results[group] = result
        
        # 对比分析
        analyzer = ResultsAnalyzer()
        analyzer.print_comparison(all_results)
        return
    
    # 运行单组
    if args.group:
        runner = BenchmarkRunner(args.group, args.days, args.project_path)
        runner.run_benchmark()
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
