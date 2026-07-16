#!/usr/bin/env python3
"""
Experiment Monitor and Alert Script

Monitors experiment progress and sends alerts for:
- API cost budget exceeded
- Task failure rate anomalies
- Experiment completion status

Usage:
    python monitor_experiments.py --group Ours-Full --budget 50.0
    python monitor_experiments.py --all-groups --budget 150.0
"""

import argparse
import json
import os
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional
import smtplib
from email.mime.text import MIMEText
from email.mime.subject import MIMESubject


class ExperimentMonitor:
    """Monitor experiment progress and send alerts."""
    
    def __init__(self, experiments_dir: str, budget_usd: float):
        self.experiments_dir = Path(experiments_dir)
        self.logs_dir = self.experiments_dir / "logs"
        self.budget_usd = budget_usd
        self.alert_thresholds = {
            "budget_warning": 0.8,  # Alert at 80% of budget
            "budget_critical": 0.95,  # Critical at 95% of budget
            "failure_rate_warning": 0.5,  # Alert if failure rate > 50%
            "failure_rate_critical": 0.7,  # Critical if failure rate > 70%
        }
    
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
    
    def calculate_metrics(self, group: str) -> Dict:
        """Calculate metrics for a group."""
        log_files = self.get_log_files(group)
        all_records = []
        
        for log_file in log_files:
            records = self.parse_log_file(log_file)
            all_records.extend(records)
        
        if not all_records:
            return {
                "group": group,
                "total_tasks": 0,
                "successful_tasks": 0,
                "failed_tasks": 0,
                "success_rate": 0.0,
                "total_api_cost": 0.0,
                "avg_tool_calls": 0.0,
                "avg_satisfaction": 0.0,
            }
        
        total_tasks = len(all_records)
        successful_tasks = sum(1 for r in all_records if r.get("success", False))
        failed_tasks = total_tasks - successful_tasks
        total_api_cost = sum(r.get("api_cost_usd", 0.0) for r in all_records)
        total_tool_calls = sum(r.get("tool_calls", 0) for r in all_records)
        total_satisfaction = sum(r.get("user_satisfaction", 0) for r in all_records)
        
        return {
            "group": group,
            "total_tasks": total_tasks,
            "successful_tasks": successful_tasks,
            "failed_tasks": failed_tasks,
            "success_rate": successful_tasks / total_tasks if total_tasks > 0 else 0.0,
            "total_api_cost": total_api_cost,
            "avg_tool_calls": total_tool_calls / total_tasks if total_tasks > 0 else 0.0,
            "avg_satisfaction": total_satisfaction / total_tasks if total_tasks > 0 else 0.0,
            "last_updated": datetime.now().isoformat(),
        }
    
    def check_alerts(self, metrics: Dict) -> List[Dict]:
        """Check for alert conditions."""
        alerts = []
        
        # Budget alerts
        if self.budget_usd > 0:
            budget_usage = metrics["total_api_cost"] / self.budget_usd
            
            if budget_usage >= self.alert_thresholds["budget_critical"]:
                alerts.append({
                    "level": "CRITICAL",
                    "type": "budget",
                    "message": f"API cost (${metrics['total_api_cost']:.2f}) exceeds {self.alert_thresholds['budget_critical']*100:.0f}% of budget (${self.budget_usd:.2f})",
                    "current_cost": metrics["total_api_cost"],
                    "budget": self.budget_usd,
                })
            elif budget_usage >= self.alert_thresholds["budget_warning"]:
                alerts.append({
                    "level": "WARNING",
                    "type": "budget",
                    "message": f"API cost (${metrics['total_api_cost']:.2f}) exceeds {self.alert_thresholds['budget_warning']*100:.0f}% of budget (${self.budget_usd:.2f})",
                    "current_cost": metrics["total_api_cost"],
                    "budget": self.budget_usd,
                })
        
        # Failure rate alerts
        if metrics["total_tasks"] > 0:
            failure_rate = metrics["failed_tasks"] / metrics["total_tasks"]
            
            if failure_rate >= self.alert_thresholds["failure_rate_critical"]:
                alerts.append({
                    "level": "CRITICAL",
                    "type": "failure_rate",
                    "message": f"Task failure rate ({failure_rate*100:.1f}%) exceeds critical threshold ({self.alert_thresholds['failure_rate_critical']*100:.0f}%)",
                    "failure_rate": failure_rate,
                })
            elif failure_rate >= self.alert_thresholds["failure_rate_warning"]:
                alerts.append({
                    "level": "WARNING",
                    "type": "failure_rate",
                    "message": f"Task failure rate ({failure_rate*100:.1f}%) exceeds warning threshold ({self.alert_thresholds['failure_rate_warning']*100:.0f}%)",
                    "failure_rate": failure_rate,
                })
        
        return alerts
    
    def send_email_alert(self, alert: Dict, recipient: str, smtp_config: Dict):
        """Send email alert."""
        subject = f"[Tokitai Experiment {alert['level']}] {alert['type'].replace('_', ' ').title()} Alert"
        
        body = f"""
Experiment Alert

Level: {alert['level']}
Type: {alert['type']}
Time: {datetime.now().isoformat()}

Message:
{alert['message']}

Details:
{json.dumps(alert, indent=2, default=str)}

---
Tokitai Experiment Monitor
"""
        
        msg = MIMEText(body)
        msg["Subject"] = subject
        msg["From"] = smtp_config.get("from_email", "noreply@tokitai")
        msg["To"] = recipient
        
        try:
            with smtplib.SMTP(smtp_config["host"], smtp_config.get("port", 587)) as server:
                server.starttls()
                server.login(smtp_config["username"], smtp_config["password"])
                server.send_message(msg)
            print(f"✓ Email alert sent to {recipient}")
        except Exception as e:
            print(f"✗ Failed to send email: {e}")
    
    def send_slack_alert(self, alert: Dict, webhook_url: str):
        """Send Slack alert."""
        color = {
            "WARNING": "warning",
            "CRITICAL": "danger",
        }.get(alert["level"], "warning")
        
        payload = {
            "attachments": [
                {
                    "color": color,
                    "title": f"Tokitai Experiment {alert['level']}",
                    "text": alert['message'],
                    "fields": [
                        {"title": "Type", "value": alert['type'], "short": True},
                        {"title": "Time", "value": datetime.now().strftime("%Y-%m-%d %H:%M:%S"), "short": True},
                    ],
                }
            ]
        }
        
        try:
            import urllib.request
            data = json.dumps(payload).encode('utf-8')
            req = urllib.request.Request(webhook_url, data=data, headers={"Content-Type": "application/json"})
            urllib.request.urlopen(req)
            print(f"✓ Slack alert sent")
        except Exception as e:
            print(f"✗ Failed to send Slack alert: {e}")
    
    def monitor(self, group: Optional[str] = None, email: Optional[str] = None,
                slack_webhook: Optional[str] = None, smtp_config: Optional[Dict] = None):
        """Run monitoring check."""
        print(f"\n{'='*60}")
        print(f"Tokitai Experiment Monitor")
        print(f"Time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print(f"Budget: ${self.budget_usd:.2f}")
        print(f"{'='*60}\n")
        
        groups = [group] if group else ["control", "ours_full", "ours_single", "ours_nocot", "ours_nofix"]
        
        all_alerts = []
        
        for group_name in groups:
            metrics = self.calculate_metrics(group_name)
            alerts = self.check_alerts(metrics)
            all_alerts.extend(alerts)
            
            # Print metrics
            print(f"Group: {group_name}")
            print(f"  Tasks: {metrics['total_tasks']} (Success: {metrics['successful_tasks']}, Failed: {metrics['failed_tasks']})")
            print(f"  Success Rate: {metrics['success_rate']*100:.1f}%")
            print(f"  API Cost: ${metrics['total_api_cost']:.2f}")
            print(f"  Avg Tool Calls: {metrics['avg_tool_calls']:.1f}")
            print(f"  Avg Satisfaction: {metrics['avg_satisfaction']:.1f}")
            
            if alerts:
                print(f"  ⚠️  Alerts: {len(alerts)}")
                for alert in alerts:
                    print(f"    [{alert['level']}] {alert['message']}")
            else:
                print(f"  ✓ No alerts")
            print()
        
        # Send alerts
        if all_alerts:
            print(f"\n🚨 Total Alerts: {len(all_alerts)}\n")
            
            for alert in all_alerts:
                if email and smtp_config:
                    self.send_email_alert(alert, email, smtp_config)
                
                if slack_webhook:
                    self.send_slack_alert(alert, slack_webhook)
        else:
            print("\n✓ All systems normal\n")
        
        return all_alerts
    
    def generate_report(self, output_path: str):
        """Generate a summary report."""
        groups = ["control", "ours_full", "ours_single", "ours_nocot", "ours_nofix"]
        report = {
            "generated_at": datetime.now().isoformat(),
            "budget_usd": self.budget_usd,
            "groups": {},
            "alerts": [],
        }
        
        for group in groups:
            metrics = self.calculate_metrics(group)
            alerts = self.check_alerts(metrics)
            report["groups"][group] = metrics
            report["alerts"].extend([{
                "group": group,
                **alert
            } for alert in alerts])
        
        with open(output_path, 'w') as f:
            json.dump(report, f, indent=2, default=str)
        
        print(f"✓ Report saved to {output_path}")
        return report


def main():
    parser = argparse.ArgumentParser(description="Monitor Tokitai experiments")
    parser.add_argument("--group", "-g", help="Monitor specific group")
    parser.add_argument("--all-groups", "-a", action="store_true", help="Monitor all groups")
    parser.add_argument("--budget", "-b", type=float, default=150.0, help="API budget in USD")
    parser.add_argument("--experiments-dir", default="experiments", help="Experiments directory")
    parser.add_argument("--email", help="Email address for alerts")
    parser.add_argument("--slack-webhook", help="Slack webhook URL for alerts")
    parser.add_argument("--report", help="Generate report to file")
    parser.add_argument("--interval", type=int, default=60, help="Monitor interval in seconds (0 for single check)")
    
    args = parser.parse_args()
    
    monitor = ExperimentMonitor(args.experiments_dir, args.budget_usd if hasattr(args, 'budget_usd') else args.budget)
    
    # SMTP config from environment
    smtp_config = {
        "host": os.getenv("SMTP_HOST", "smtp.gmail.com"),
        "port": int(os.getenv("SMTP_PORT", "587")),
        "username": os.getenv("SMTP_USERNAME"),
        "password": os.getenv("SMTP_PASSWORD"),
        "from_email": os.getenv("SMTP_FROM_EMAIL", "noreply@tokitai"),
    }
    
    if args.interval > 0:
        print(f"Starting continuous monitoring (interval: {args.interval}s)...")
        print("Press Ctrl+C to stop\n")
        
        try:
            while True:
                monitor.monitor(
                    group=args.group if not args.all_groups else None,
                    email=args.email,
                    slack_webhook=args.slack_webhook,
                    smtp_config=smtp_config if smtp_config["username"] else None,
                )
                
                if args.report:
                    monitor.generate_report(args.report)
                
                import time
                time.sleep(args.interval)
        except KeyboardInterrupt:
            print("\n\nMonitoring stopped.")
    else:
        monitor.monitor(
            group=args.group if not args.all_groups else None,
            email=args.email,
            slack_webhook=args.slack_webhook,
            smtp_config=smtp_config if smtp_config["username"] else None,
        )
        
        if args.report:
            monitor.generate_report(args.report)


if __name__ == "__main__":
    main()
