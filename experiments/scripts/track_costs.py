#!/usr/bin/env python3
"""
API Cost Tracker for Atlas experiments

Tracks API costs in real-time and projects total experiment cost.

Usage:
    python track_costs.py --group Ours-Full --budget 50.0
    python track_costs.py --all-groups --budget 150.0 --project-cost
"""

import argparse
import json
import os
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional


class CostTracker:
    """Track and project API costs for experiments."""
    
    def __init__(self, experiments_dir: str):
        self.experiments_dir = Path(experiments_dir)
        self.logs_dir = self.experiments_dir / "logs"
    
    def get_log_files(self, group: Optional[str] = None) -> List[Path]:
        """Get all log files for a group or all groups."""
        if group:
            log_dir = self.logs_dir / group.lower().replace("-", "_")
            if log_dir.exists():
                return list(log_dir.glob("*.jsonl"))
            return []
        
        all_logs = []
        if self.logs_dir.exists():
            for group_dir in self.logs_dir.iterdir():
                if group_dir.is_dir():
                    all_logs.extend(group_dir.glob("*.jsonl"))
        return all_logs
    
    def parse_log_file(self, log_file: Path) -> List[dict]:
        """Parse a JSONL log file."""
        records = []
        try:
            with open(log_file, 'r') as f:
                for line in f:
                    line = line.strip()
                    if line:
                        try:
                            records.append(json.loads(line))
                        except json.JSONDecodeError:
                            continue
        except Exception as e:
            print(f"Error reading {log_file}: {e}")
        return records
    
    def calculate_costs(self, group: Optional[str] = None) -> Dict[str, Dict]:
        """Calculate costs for a group or all groups."""
        groups = [group] if group else ["control", "ours_full", "ours_single", "ours_nocot", "ours_nofix"]
        
        cost_data = {}
        
        for group_name in groups:
            log_files = self.get_log_files(group_name)
            all_records = []
            
            for log_file in log_files:
                records = self.parse_log_file(log_file)
                all_records.extend(records)
            
            if not all_records:
                cost_data[group_name] = {
                    "total_cost": 0.0,
                    "task_count": 0,
                    "avg_cost_per_task": 0.0,
                    "evolution_cycles": 0,
                    "evolution_cost": 0.0,
                }
                continue
            
            # Calculate task execution costs
            task_cost = sum(r.get("api_cost_usd", 0.0) for r in all_records if "task_id" in r)
            task_count = len([r for r in all_records if "task_id" in r])
            
            # Calculate evolution cycle costs
            evolution_records = [r for r in all_records if "cycle_id" in r]
            evolution_cost = sum(r.get("api_cost_usd", 0.0) for r in evolution_records)
            evolution_cycles = len(evolution_records)
            
            total_cost = task_cost + evolution_cost
            
            cost_data[group_name] = {
                "total_cost": total_cost,
                "task_cost": task_cost,
                "task_count": task_count,
                "avg_cost_per_task": task_cost / task_count if task_count > 0 else 0.0,
                "evolution_cycles": evolution_cycles,
                "evolution_cost": evolution_cost,
                "avg_cost_per_cycle": evolution_cost / evolution_cycles if evolution_cycles > 0 else 0.0,
            }
        
        return cost_data
    
    def project_total_cost(self, cost_data: Dict, experiment_days: int = 30) -> Dict[str, float]:
        """Project total experiment cost based on current spending."""
        projections = {}
        
        for group, data in cost_data.items():
            if data["task_count"] == 0:
                projections[group] = 0.0
                continue
            
            # Assume costs are proportional to time
            # This is a simple projection - adjust based on actual experiment design
            daily_cost = data["total_cost"]  # Assuming 1 day of data
            projected_cost = daily_cost * experiment_days
            
            projections[group] = projected_cost
        
        return projections
    
    def print_report(self, cost_data: Dict, budget: Optional[float] = None,
                     project: bool = False, experiment_days: int = 30):
        """Print a cost report."""
        print("\n" + "=" * 70)
        print("ATLAS EXPERIMENT API COST REPORT")
        print(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print("=" * 70 + "\n")
        
        total_all_groups = 0.0
        
        for group, data in cost_data.items():
            print(f"Group: {group}")
            print("-" * 40)
            print(f"  Task Execution Cost:    ${data['task_cost']:>10.2f}")
            print(f"  Evolution Cycle Cost:   ${data['evolution_cost']:>10.2f}")
            print(f"  Total Cost:             ${data['total_cost']:>10.2f}")
            print(f"  Task Count:             {data['task_count']:>10}")
            print(f"  Avg Cost per Task:      ${data['avg_cost_per_task']:>10.4f}")
            print(f"  Evolution Cycles:       {data['evolution_cycles']:>10}")
            print(f"  Avg Cost per Cycle:     ${data['avg_cost_per_cycle']:>10.4f}")
            
            if budget:
                budget_usage = (data['total_cost'] / budget) * 100
                status = "✓" if budget_usage < 80 else "⚠️" if budget_usage < 100 else "🚨"
                print(f"  Budget Usage:           {status} {budget_usage:>9.1f}%")
            
            print()
            total_all_groups += data['total_cost']
        
        print("=" * 70)
        print(f"TOTAL (All Groups):      ${total_all_groups:>10.2f}")
        
        if budget:
            total_budget = budget * len(cost_data)  # Assuming same budget per group
            total_usage = (total_all_groups / total_budget) * 100 if total_budget > 0 else 0
            status = "✓" if total_usage < 80 else "⚠️" if total_usage < 100 else "🚨"
            print(f"Total Budget Usage:      {status} {total_usage:>9.1f}%")
        
        if project:
            print("\n" + "=" * 70)
            print(f"PROJECTED COST ({experiment_days} DAYS)")
            print("=" * 70 + "\n")
            
            projections = self.project_total_cost(cost_data, experiment_days)
            total_projected = 0.0
            
            for group, projected in projections.items():
                print(f"  {group}: ${projected:>10.2f}")
                total_projected += projected
            
            print("-" * 40)
            print(f"  TOTAL:   ${total_projected:>10.2f}")
            
            if budget:
                projected_budget = budget * len(projections)
                usage = (total_projected / projected_budget) * 100 if projected_budget > 0 else 0
                status = "✓" if usage < 80 else "⚠️" if usage < 100 else "🚨"
                print(f"  Budget Usage: {status} {usage:>9.1f}%")
        
        print("\n" + "=" * 70 + "\n")
    
    def export_costs(self, output_path: str, cost_data: Dict):
        """Export cost data to JSON file."""
        report = {
            "generated_at": datetime.now().isoformat(),
            "groups": cost_data,
            "total_cost": sum(d["total_cost"] for d in cost_data.values()),
        }
        
        with open(output_path, 'w') as f:
            json.dump(report, f, indent=2)
        
        print(f"✓ Cost data exported to {output_path}")


def main():
    parser = argparse.ArgumentParser(description="Track API costs for Atlas experiments")
    parser.add_argument("--group", "-g", help="Track specific group")
    parser.add_argument("--all-groups", "-a", action="store_true", help="Track all groups")
    parser.add_argument("--budget", "-b", type=float, default=50.0, help="Budget per group in USD")
    parser.add_argument("--experiments-dir", default="experiments", help="Experiments directory")
    parser.add_argument("--project-cost", "-p", action="store_true", help="Project total experiment cost")
    parser.add_argument("--experiment-days", type=int, default=30, help="Number of days for projection")
    parser.add_argument("--export", help="Export cost data to JSON file")
    
    args = parser.parse_args()
    
    tracker = CostTracker(args.experiments_dir)
    cost_data = tracker.calculate_costs(args.group if not args.all_groups else None)
    
    tracker.print_report(
        cost_data,
        budget=args.budget,
        project=args.project_cost,
        experiment_days=args.experiment_days,
    )
    
    if args.export:
        tracker.export_costs(args.export, cost_data)


if __name__ == "__main__":
    main()
