#!/usr/bin/env python3
"""
QA Badge Generation Script for CI/CD

Generates SVG badges showing QA status for README and documentation.
"""

import json
import sys
from pathlib import Path
from typing import Dict, Any

def load_latest_qa_report(reports_dir: str) -> Dict[str, Any]:
    """Load the most recent QA report"""
    reports_path = Path(reports_dir)
    
    if not reports_path.exists():
        return {}
    
    latest_report = None
    latest_time = ""
    
    for report_file in reports_path.rglob("*.json"):
        if "qa-report" in report_file.name:
            try:
                with open(report_file, 'r') as f:
                    report = json.load(f)
                    timestamp = report.get('timestamp', '')
                    if timestamp > latest_time:
                        latest_time = timestamp
                        latest_report = report
            except Exception:
                continue
    
    return latest_report or {}

def generate_badge_svg(status: str, pass_rate: float, message: str = "") -> str:
    """Generate SVG badge for QA status"""
    
    # Color scheme based on status
    colors = {
        "PASS": "#4c1",
        "PASS_WITH_WARNINGS": "#fe7d37",
        "FAIL": "#e05d44",
        "ERROR": "#e05d44",
        "UNKNOWN": "#9f9f9f"
    }
    
    color = colors.get(status, colors["UNKNOWN"])
    
    # Badge text
    if message:
        badge_text = message
    else:
        if status == "PASS":
            badge_text = f"passing ({pass_rate:.0%})"
        elif status == "PASS_WITH_WARNINGS":
            badge_text = f"warnings ({pass_rate:.0%})"
        elif status == "FAIL":
            badge_text = f"failing ({pass_rate:.0%})"
        elif status == "ERROR":
            badge_text = "error"
        else:
            badge_text = "unknown"
    
    # Calculate text width (approximate)
    label_width = len("QA") * 7 + 10
    message_width = len(badge_text) * 7 + 10
    total_width = label_width + message_width
    
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{total_width}" height="20" role="img" aria-label="QA: {badge_text}">
    <title>QA: {badge_text}</title>
    <linearGradient id="s" x2="0" y2="100%">
        <stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
        <stop offset="1" stop-opacity=".1"/>
    </linearGradient>
    <clipPath id="r">
        <rect width="{total_width}" height="20" rx="3" fill="#fff"/>
    </clipPath>
    <g clip-path="url(#r)">
        <rect width="{label_width}" height="20" fill="#555"/>
        <rect x="{label_width}" width="{message_width}" height="20" fill="{color}"/>
        <rect width="{total_width}" height="20" fill="url(#s)"/>
    </g>
    <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" text-rendering="geometricPrecision" font-size="110">
        <text aria-hidden="true" x="{label_width//2 + 5}" y="15" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="20">QA</text>
        <text x="{label_width//2 + 5}" y="14" transform="scale(.1)" fill="#fff" textLength="20">QA</text>
        <text aria-hidden="true" x="{label_width + message_width//2}" y="15" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="{(message_width-10)*10}">{badge_text}</text>
        <text x="{label_width + message_width//2}" y="14" transform="scale(.1)" fill="#fff" textLength="{(message_width-10)*10}">{badge_text}</text>
    </g>
</svg>'''
    
    return svg

def main():
    """Main entry point"""
    if len(sys.argv) != 2:
        print("Usage: python3 generate_qa_badge.py <reports_directory>", file=sys.stderr)
        sys.exit(1)
    
    reports_dir = sys.argv[1]
    
    # Load latest QA report
    report = load_latest_qa_report(reports_dir)
    
    if not report:
        # No reports found
        svg = generate_badge_svg("UNKNOWN", 0, "no reports")
    else:
        # Determine status
        total_tests = report.get('total_tests', 0)
        passed = report.get('passed', 0)
        failed = report.get('failed', 0)
        errors = report.get('errors', 0)
        
        if total_tests == 0:
            status = "UNKNOWN"
            pass_rate = 0
        else:
            pass_rate = passed / total_tests
            
            if errors > 0:
                status = "ERROR"
            elif failed > 0:
                status = "PASS_WITH_WARNINGS" if pass_rate >= 0.8 else "FAIL"
            else:
                status = "PASS"
        
        svg = generate_badge_svg(status, pass_rate)
    
    print(svg)

if __name__ == "__main__":
    main()
