#!/usr/bin/env python3
"""
QA Dashboard for Dattavani ASR Rust Port

Provides a real-time dashboard for monitoring QA test results,
performance metrics, and quality trends over time.

Author: QA Automation System
Version: 1.0.0
"""

import json
import os
import time
from pathlib import Path
from datetime import datetime, timedelta
from typing import List, Dict, Any
import argparse

class QADashboard:
    """QA Dashboard for monitoring test results and trends"""
    
    def __init__(self, qa_root: str):
        self.qa_root = Path(qa_root)
        self.reports_dir = self.qa_root / "reports"
        
    def load_reports(self, days: int = 7) -> List[Dict[str, Any]]:
        """Load QA reports from the last N days"""
        reports = []
        cutoff_date = datetime.now() - timedelta(days=days)
        
        if not self.reports_dir.exists():
            return reports
        
        for report_file in self.reports_dir.glob("qa_report_*.json"):
            try:
                with open(report_file, 'r') as f:
                    report = json.load(f)
                
                # Parse timestamp
                report_time = datetime.fromisoformat(report['timestamp'].replace('Z', '+00:00'))
                if report_time.replace(tzinfo=None) >= cutoff_date:
                    reports.append(report)
            except Exception as e:
                print(f"Warning: Could not load report {report_file}: {e}")
        
        # Sort by timestamp
        reports.sort(key=lambda x: x['timestamp'], reverse=True)
        return reports
    
    def get_latest_report(self) -> Dict[str, Any]:
        """Get the most recent QA report"""
        reports = self.load_reports(days=1)
        return reports[0] if reports else {}
    
    def calculate_trends(self, reports: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Calculate trends from historical reports"""
        trends = {
            "trend": "insufficient_data",
            "pass_rate_current": 0,
            "pass_rate_average": 0,
            "startup_time_current": 0,
            "startup_time_average": 0,
            "total_reports": len(reports)
        }
        
        if len(reports) < 1:
            return trends
        
        # Calculate pass rate trend
        pass_rates = [r['passed'] / r['total_tests'] for r in reports if r['total_tests'] > 0]
        
        if len(pass_rates) >= 2:
            recent_avg = sum(pass_rates[:3]) / min(3, len(pass_rates))
            older_avg = sum(pass_rates[3:6]) / max(1, min(3, len(pass_rates[3:])))
            trend = "improving" if recent_avg > older_avg else "declining" if recent_avg < older_avg else "stable"
        else:
            trend = "stable"
        
        # Performance trends
        startup_times = []
        for report in reports:
            for result in report.get('results', []):
                if result['name'] == 'Startup Performance' and result['status'] == 'PASS':
                    startup_times.append(result.get('details', {}).get('average_startup', 0))
        
        trends.update({
            "trend": trend,
            "pass_rate_current": pass_rates[0] if pass_rates else 0,
            "pass_rate_average": sum(pass_rates) / len(pass_rates) if pass_rates else 0,
            "startup_time_current": startup_times[0] if startup_times else 0,
            "startup_time_average": sum(startup_times) / len(startup_times) if startup_times else 0,
        })
        
        return trends
    
    def generate_dashboard_text(self) -> str:
        """Generate text-based dashboard"""
        latest_report = self.get_latest_report()
        reports = self.load_reports(days=7)
        trends = self.calculate_trends(reports)
        
        if not latest_report:
            return "❌ No QA reports found. Run QA tests first."
        
        # Header
        dashboard = "🔍 DATTAVANI ASR QA DASHBOARD\n"
        dashboard += "=" * 50 + "\n\n"
        
        # Current Status
        status_icon = "✅" if latest_report.get('summary', {}).get('overall_status') == 'PASS' else "❌"
        dashboard += f"📊 CURRENT STATUS: {status_icon} {latest_report.get('summary', {}).get('overall_status', 'UNKNOWN')}\n"
        dashboard += f"📅 Last Run: {latest_report.get('timestamp', 'Unknown')[:19]}\n"
        dashboard += f"⏱️  Duration: {latest_report.get('duration', 0):.2f}s\n\n"
        
        # Test Results Summary
        dashboard += "🧪 TEST RESULTS\n"
        dashboard += "-" * 20 + "\n"
        dashboard += f"Total Tests: {latest_report.get('total_tests', 0)}\n"
        dashboard += f"✅ Passed: {latest_report.get('passed', 0)}\n"
        dashboard += f"❌ Failed: {latest_report.get('failed', 0)}\n"
        dashboard += f"⏭️  Skipped: {latest_report.get('skipped', 0)}\n"
        dashboard += f"🔥 Errors: {latest_report.get('errors', 0)}\n"
        
        pass_rate = latest_report.get('passed', 0) / max(1, latest_report.get('total_tests', 1))
        dashboard += f"📈 Pass Rate: {pass_rate:.1%}\n\n"
        
        # Performance Metrics
        dashboard += "⚡ PERFORMANCE\n"
        dashboard += "-" * 20 + "\n"
        
        startup_result = None
        for result in latest_report.get('results', []):
            if result['name'] == 'Startup Performance':
                startup_result = result
                break
        
        if startup_result and startup_result['status'] == 'PASS':
            startup_time = startup_result.get('details', {}).get('average_startup', 0)
            dashboard += f"🚀 Startup Time: {startup_time:.3f}s\n"
            
            if startup_time < 0.1:
                dashboard += "   Rating: ⭐⭐⭐⭐⭐ EXCELLENT\n"
            elif startup_time < 1.0:
                dashboard += "   Rating: ⭐⭐⭐⭐ VERY GOOD\n"
            elif startup_time < 3.0:
                dashboard += "   Rating: ⭐⭐⭐ GOOD\n"
            else:
                dashboard += "   Rating: ⭐⭐ NEEDS IMPROVEMENT\n"
        else:
            dashboard += "🚀 Startup Time: Not measured\n"
        
        # Binary Info
        binary_size = latest_report.get('summary', {}).get('binary_size_mb', 0)
        dashboard += f"📦 Binary Size: {binary_size:.1f} MB\n\n"
        
        # Trends
        dashboard += "📈 TRENDS (7 days)\n"
        dashboard += "-" * 20 + "\n"
        dashboard += f"📊 Trend: {trends['trend'].upper()}\n"
        dashboard += f"📋 Reports: {trends['total_reports']}\n"
        
        if trends['pass_rate_current'] > 0:
            dashboard += f"✅ Current Pass Rate: {trends['pass_rate_current']:.1%}\n"
            dashboard += f"📊 Average Pass Rate: {trends['pass_rate_average']:.1%}\n"
        
        # Recent Failures
        dashboard += "\n🔍 RECENT ISSUES\n"
        dashboard += "-" * 20 + "\n"
        
        failed_tests = [r for r in latest_report.get('results', []) if r['status'] in ['FAIL', 'ERROR']]
        if failed_tests:
            for test in failed_tests:
                dashboard += f"❌ {test['name']}: {test['message']}\n"
        else:
            dashboard += "✅ No recent failures\n"
        
        # Recommendations
        dashboard += "\n💡 RECOMMENDATIONS\n"
        dashboard += "-" * 20 + "\n"
        
        recommendations = []
        
        if latest_report.get('failed', 0) > 0:
            recommendations.append("🔧 Address failing tests")
        
        if pass_rate < 0.9:
            recommendations.append("📈 Improve test pass rate")
        
        clippy_failed = any(r['name'] == 'Code Quality (Clippy)' and r['status'] == 'FAIL' 
                           for r in latest_report.get('results', []))
        if clippy_failed:
            recommendations.append("🧹 Clean up code quality warnings")
        
        if not recommendations:
            recommendations.append("✨ All systems looking good!")
        
        for rec in recommendations:
            dashboard += f"{rec}\n"
        
        dashboard += "\n" + "=" * 50 + "\n"
        dashboard += f"🤖 Generated by QA Dashboard at {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
        
        return dashboard
    
    def watch_mode(self, interval: int = 30):
        """Run dashboard in watch mode with auto-refresh"""
        try:
            while True:
                # Clear screen (works on most terminals)
                os.system('clear' if os.name == 'posix' else 'cls')
                
                print(self.generate_dashboard_text())
                print(f"\n🔄 Auto-refreshing every {interval}s... (Ctrl+C to exit)")
                
                time.sleep(interval)
        except KeyboardInterrupt:
            print("\n👋 Dashboard stopped.")
    
    def export_metrics(self, format: str = "json") -> str:
        """Export metrics for external monitoring systems"""
        latest_report = self.get_latest_report()
        reports = self.load_reports(days=30)
        trends = self.calculate_trends(reports)
        
        metrics = {
            "timestamp": datetime.now().isoformat(),
            "status": latest_report.get('summary', {}).get('overall_status', 'UNKNOWN'),
            "pass_rate": latest_report.get('passed', 0) / max(1, latest_report.get('total_tests', 1)),
            "total_tests": latest_report.get('total_tests', 0),
            "failed_tests": latest_report.get('failed', 0),
            "duration": latest_report.get('duration', 0),
            "binary_size_mb": latest_report.get('summary', {}).get('binary_size_mb', 0),
            "trend": trends['trend'],
            "reports_count": trends['total_reports']
        }
        
        # Add performance metrics
        for result in latest_report.get('results', []):
            if result['name'] == 'Startup Performance' and result['status'] == 'PASS':
                metrics['startup_time'] = result.get('details', {}).get('average_startup', 0)
                break
        
        if format == "json":
            return json.dumps(metrics, indent=2)
        elif format == "prometheus":
            # Prometheus format
            prom_metrics = []
            prom_metrics.append(f'dattavani_asr_pass_rate {metrics["pass_rate"]}')
            prom_metrics.append(f'dattavani_asr_total_tests {metrics["total_tests"]}')
            prom_metrics.append(f'dattavani_asr_failed_tests {metrics["failed_tests"]}')
            prom_metrics.append(f'dattavani_asr_duration_seconds {metrics["duration"]}')
            prom_metrics.append(f'dattavani_asr_binary_size_mb {metrics["binary_size_mb"]}')
            if "startup_time" in metrics:
                prom_metrics.append(f'dattavani_asr_startup_time_seconds {metrics["startup_time"]}')
            return "\n".join(prom_metrics)
        
        return str(metrics)

def main():
    """Main entry point for QA dashboard"""
    parser = argparse.ArgumentParser(description="QA Dashboard for Dattavani ASR Rust Port")
    parser.add_argument("--qa-root", default="qa-agent", help="QA root directory")
    parser.add_argument("--watch", action="store_true", help="Run in watch mode")
    parser.add_argument("--interval", type=int, default=30, help="Refresh interval for watch mode")
    parser.add_argument("--export", choices=["json", "prometheus"], help="Export metrics format")
    parser.add_argument("--output", help="Output file for exported metrics")
    
    args = parser.parse_args()
    
    dashboard = QADashboard(args.qa_root)
    
    if args.export:
        metrics = dashboard.export_metrics(args.export)
        if args.output:
            with open(args.output, 'w') as f:
                f.write(metrics)
            print(f"Metrics exported to {args.output}")
        else:
            print(metrics)
    elif args.watch:
        dashboard.watch_mode(args.interval)
    else:
        print(dashboard.generate_dashboard_text())

if __name__ == "__main__":
    main()
