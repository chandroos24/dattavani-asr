#!/usr/bin/env python3
"""
QA Results Aggregation Script for CI/CD

Aggregates QA results from multiple test runs and generates a summary report.
"""

import json
import sys
import os
from pathlib import Path
from typing import List, Dict, Any
from datetime import datetime

def load_qa_reports(reports_dir: str) -> List[Dict[str, Any]]:
    """Load all QA reports from the reports directory"""
    reports = []
    reports_path = Path(reports_dir)
    
    if not reports_path.exists():
        return reports
    
    # Find all QA report files
    for report_file in reports_path.rglob("*.json"):
        if "qa-report" in report_file.name:
            try:
                with open(report_file, 'r') as f:
                    report = json.load(f)
                    report['source_file'] = str(report_file)
                    reports.append(report)
            except Exception as e:
                print(f"Warning: Could not load {report_file}: {e}", file=sys.stderr)
    
    return reports

def aggregate_results(reports: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Aggregate results from multiple QA reports"""
    if not reports:
        return {
            "status": "NO_REPORTS",
            "total_runs": 0,
            "summary": "No QA reports found"
        }
    
    # Aggregate metrics
    total_runs = len(reports)
    total_tests = sum(r.get('total_tests', 0) for r in reports)
    total_passed = sum(r.get('passed', 0) for r in reports)
    total_failed = sum(r.get('failed', 0) for r in reports)
    total_skipped = sum(r.get('skipped', 0) for r in reports)
    total_errors = sum(r.get('errors', 0) for r in reports)
    
    # Calculate overall pass rate
    overall_pass_rate = total_passed / max(1, total_tests)
    
    # Determine overall status
    if total_errors > 0:
        status = "ERROR"
    elif total_failed > 0:
        if overall_pass_rate >= 0.8:
            status = "PASS_WITH_WARNINGS"
        else:
            status = "FAIL"
    else:
        status = "PASS"
    
    # Find common failures
    failure_counts = {}
    for report in reports:
        for result in report.get('results', []):
            if result['status'] in ['FAIL', 'ERROR']:
                test_name = result['name']
                failure_counts[test_name] = failure_counts.get(test_name, 0) + 1
    
    # Performance metrics
    startup_times = []
    binary_sizes = []
    for report in reports:
        for result in report.get('results', []):
            if result['name'] == 'Startup Performance' and result['status'] == 'PASS':
                details = result.get('details', {})
                if 'average_startup' in details:
                    startup_times.append(details['average_startup'])
        
        summary = report.get('summary', {})
        if 'binary_size_mb' in summary:
            binary_sizes.append(summary['binary_size_mb'])
    
    return {
        "status": status,
        "timestamp": datetime.now().isoformat(),
        "total_runs": total_runs,
        "total_tests": total_tests,
        "total_passed": total_passed,
        "total_failed": total_failed,
        "total_skipped": total_skipped,
        "total_errors": total_errors,
        "overall_pass_rate": overall_pass_rate,
        "common_failures": dict(sorted(failure_counts.items(), key=lambda x: x[1], reverse=True)),
        "performance": {
            "avg_startup_time": sum(startup_times) / len(startup_times) if startup_times else 0,
            "avg_binary_size_mb": sum(binary_sizes) / len(binary_sizes) if binary_sizes else 0,
            "startup_samples": len(startup_times),
            "binary_samples": len(binary_sizes)
        },
        "platforms_tested": list(set(r.get('summary', {}).get('platform', 'unknown') for r in reports))
    }

def generate_markdown_report(aggregated: Dict[str, Any]) -> str:
    """Generate a markdown report from aggregated results"""
    status = aggregated['status']
    
    # Status emoji
    status_emoji = {
        "PASS": "✅",
        "PASS_WITH_WARNINGS": "⚠️",
        "FAIL": "❌",
        "ERROR": "🔥",
        "NO_REPORTS": "❓"
    }.get(status, "❓")
    
    report = f"""# QA Aggregation Report

## {status_emoji} Overall Status: {status}

**Generated**: {aggregated['timestamp'][:19]}  
**Test Runs**: {aggregated['total_runs']}  
**Overall Pass Rate**: {aggregated['overall_pass_rate']:.1%}

## 📊 Test Summary

| Metric | Count |
|--------|-------|
| Total Tests | {aggregated['total_tests']} |
| ✅ Passed | {aggregated['total_passed']} |
| ❌ Failed | {aggregated['total_failed']} |
| ⏭️ Skipped | {aggregated['total_skipped']} |
| 🔥 Errors | {aggregated['total_errors']} |

"""

    # Performance section
    perf = aggregated['performance']
    if perf['startup_samples'] > 0:
        report += f"""## ⚡ Performance Metrics

| Metric | Value |
|--------|-------|
| Average Startup Time | {perf['avg_startup_time']:.3f}s |
| Average Binary Size | {perf['avg_binary_size_mb']:.1f} MB |
| Performance Samples | {perf['startup_samples']} |

"""

    # Common failures
    if aggregated['common_failures']:
        report += "## 🔍 Common Failures\n\n"
        for test_name, count in list(aggregated['common_failures'].items())[:5]:
            report += f"- **{test_name}**: Failed in {count}/{aggregated['total_runs']} runs\n"
        report += "\n"

    # Platforms tested
    if aggregated['platforms_tested']:
        platforms = [p for p in aggregated['platforms_tested'] if p != 'unknown']
        if platforms:
            report += f"## 🖥️ Platforms Tested\n\n"
            for platform in platforms:
                report += f"- {platform}\n"
            report += "\n"

    # Recommendations
    report += "## 💡 Recommendations\n\n"
    
    if status == "PASS":
        report += "✨ All QA checks passed! The code is ready for deployment.\n"
    elif status == "PASS_WITH_WARNINGS":
        report += "⚠️ QA passed with warnings. Consider addressing the failing tests before deployment.\n"
    elif status == "FAIL":
        report += "❌ QA checks failed. Please fix the failing tests before merging.\n"
    elif status == "ERROR":
        report += "🔥 QA encountered errors. Please check the test configuration and environment.\n"
    else:
        report += "❓ No QA reports found. Please ensure QA tests are running properly.\n"

    return report

def main():
    """Main entry point"""
    if len(sys.argv) != 2:
        print("Usage: python3 aggregate_qa_results.py <reports_directory>", file=sys.stderr)
        sys.exit(1)
    
    reports_dir = sys.argv[1]
    
    # Load and aggregate reports
    reports = load_qa_reports(reports_dir)
    aggregated = aggregate_results(reports)
    
    # Generate markdown report
    markdown_report = generate_markdown_report(aggregated)
    print(markdown_report)
    
    # Exit with appropriate code
    exit_code = 0 if aggregated['status'] in ['PASS', 'PASS_WITH_WARNINGS'] else 1
    sys.exit(exit_code)

if __name__ == "__main__":
    main()
