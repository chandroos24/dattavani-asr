#!/usr/bin/env python3
"""
QA Agent for Dattavani ASR Rust Port

This comprehensive QA agent validates the functionality, performance, and quality
of the Dattavani ASR Rust application through automated testing and verification.

Author: QA Automation System
Version: 1.0.0
"""

import os
import sys
import json
import time
import subprocess
import tempfile
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Optional, Tuple, Any
from dataclasses import dataclass, asdict
import argparse
import logging

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('qa-agent/reports/qa_agent.log'),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)

@dataclass
class TestResult:
    """Test result data structure"""
    name: str
    category: str
    status: str  # PASS, FAIL, SKIP, ERROR
    duration: float
    message: str
    details: Optional[Dict[str, Any]] = None
    timestamp: str = ""
    
    def __post_init__(self):
        if not self.timestamp:
            self.timestamp = datetime.now().isoformat()

@dataclass
class QAReport:
    """QA report data structure"""
    timestamp: str
    total_tests: int
    passed: int
    failed: int
    skipped: int
    errors: int
    duration: float
    results: List[TestResult]
    summary: Dict[str, Any]
    
    def __post_init__(self):
        if not self.timestamp:
            self.timestamp = datetime.now().isoformat()

class DattavaniQAAgent:
    """Main QA Agent for Dattavani ASR Rust Port"""
    
    def __init__(self, project_root: str):
        self.project_root = Path(project_root)
        self.binary_path = self.project_root / "target" / "release" / "dattavani-asr"
        self.qa_root = self.project_root / "qa-agent"
        self.results: List[TestResult] = []
        self.start_time = time.time()
        
        # Ensure QA directories exist
        for dir_name in ["tests", "scripts", "reports", "config", "data"]:
            (self.qa_root / dir_name).mkdir(exist_ok=True)
    
    def run_command(self, cmd: List[str], timeout: int = 30, cwd: Optional[str] = None) -> Tuple[int, str, str]:
        """Run a command and return exit code, stdout, stderr"""
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout,
                cwd=cwd or self.project_root
            )
            return result.returncode, result.stdout, result.stderr
        except subprocess.TimeoutExpired:
            return -1, "", f"Command timed out after {timeout} seconds"
        except Exception as e:
            return -1, "", str(e)
    
    def add_result(self, name: str, category: str, status: str, message: str, 
                   duration: float = 0.0, details: Optional[Dict] = None):
        """Add a test result"""
        result = TestResult(
            name=name,
            category=category,
            status=status,
            duration=duration,
            message=message,
            details=details or {}
        )
        self.results.append(result)
        logger.info(f"{status}: {name} - {message}")
    
    def test_binary_exists(self) -> bool:
        """Test if the binary exists and is executable"""
        start_time = time.time()
        
        if not self.binary_path.exists():
            self.add_result(
                "Binary Existence",
                "build",
                "FAIL",
                f"Binary not found at {self.binary_path}",
                time.time() - start_time
            )
            return False
        
        if not os.access(self.binary_path, os.X_OK):
            self.add_result(
                "Binary Executable",
                "build",
                "FAIL",
                "Binary exists but is not executable",
                time.time() - start_time
            )
            return False
        
        self.add_result(
            "Binary Existence",
            "build",
            "PASS",
            f"Binary found and executable at {self.binary_path}",
            time.time() - start_time
        )
        return True
    
    def test_help_command(self) -> bool:
        """Test the --help command"""
        start_time = time.time()
        
        exit_code, stdout, stderr = self.run_command([str(self.binary_path), "--help"])
        duration = time.time() - start_time
        
        if exit_code != 0:
            self.add_result(
                "Help Command",
                "cli",
                "FAIL",
                f"Help command failed with exit code {exit_code}",
                duration,
                {"stderr": stderr}
            )
            return False
        
        # Check for expected help content
        expected_content = [
            "High-performance Automatic Speech Recognition",
            "Commands:",
            "stream-process",
            "stream-batch",
            "supported-formats"
        ]
        
        missing_content = [content for content in expected_content if content not in stdout]
        
        if missing_content:
            self.add_result(
                "Help Command Content",
                "cli",
                "FAIL",
                f"Missing expected content: {missing_content}",
                duration,
                {"stdout": stdout[:500]}
            )
            return False
        
        self.add_result(
            "Help Command",
            "cli",
            "PASS",
            "Help command executed successfully with expected content",
            duration,
            {"stdout_length": len(stdout)}
        )
        return True
    
    def test_version_command(self) -> bool:
        """Test the --version command"""
        start_time = time.time()
        
        exit_code, stdout, stderr = self.run_command([str(self.binary_path), "--version"])
        duration = time.time() - start_time
        
        if exit_code != 0:
            self.add_result(
                "Version Command",
                "cli",
                "FAIL",
                f"Version command failed with exit code {exit_code}",
                duration,
                {"stderr": stderr}
            )
            return False
        
        # Check for version information
        if "dattavani-asr" not in stdout.lower():
            self.add_result(
                "Version Command Content",
                "cli",
                "FAIL",
                "Version output doesn't contain application name",
                duration,
                {"stdout": stdout}
            )
            return False
        
        self.add_result(
            "Version Command",
            "cli",
            "PASS",
            "Version command executed successfully",
            duration,
            {"version_output": stdout.strip()}
        )
        return True
    
    def test_supported_formats(self) -> bool:
        """Test the supported-formats command"""
        start_time = time.time()
        
        exit_code, stdout, stderr = self.run_command([str(self.binary_path), "supported-formats"])
        duration = time.time() - start_time
        
        if exit_code != 0:
            self.add_result(
                "Supported Formats Command",
                "cli",
                "FAIL",
                f"Supported formats command failed with exit code {exit_code}",
                duration,
                {"stderr": stderr}
            )
            return False
        
        # Check for expected format categories in stderr (where logs go)
        expected_formats = ["mp4", "mp3", "wav", "avi", "English", "Spanish"]
        found_formats = [fmt for fmt in expected_formats if fmt in stderr]
        
        if len(found_formats) < 4:  # Should find at least 4 of the expected formats
            self.add_result(
                "Supported Formats Content",
                "cli",
                "FAIL",
                f"Expected format information not found. Found: {found_formats}",
                duration,
                {"stderr": stderr[:1000]}
            )
            return False
        
        self.add_result(
            "Supported Formats Command",
            "cli",
            "PASS",
            f"Supported formats command executed successfully. Found formats: {found_formats}",
            duration,
            {"formats_found": len(found_formats)}
        )
        return True
    
    def test_generate_config(self) -> bool:
        """Test the generate-config command"""
        start_time = time.time()
        
        # Use a temporary config file
        with tempfile.NamedTemporaryFile(suffix=".toml", delete=False) as tmp_file:
            config_path = tmp_file.name
        
        try:
            exit_code, stdout, stderr = self.run_command([
                str(self.binary_path), 
                "generate-config", 
                "--output", 
                config_path
            ])
            duration = time.time() - start_time
            
            if exit_code != 0:
                self.add_result(
                    "Generate Config Command",
                    "cli",
                    "FAIL",
                    f"Generate config command failed with exit code {exit_code}",
                    duration,
                    {"stderr": stderr}
                )
                return False
            
            # Check if config file was created
            if not os.path.exists(config_path):
                self.add_result(
                    "Generate Config File Creation",
                    "cli",
                    "FAIL",
                    "Config file was not created",
                    duration
                )
                return False
            
            # Check config file content
            with open(config_path, 'r') as f:
                config_content = f.read()
            
            expected_sections = ["[google]", "[whisper]", "[processing]", "[logging]"]
            missing_sections = [section for section in expected_sections if section not in config_content]
            
            if missing_sections:
                self.add_result(
                    "Generate Config Content",
                    "cli",
                    "FAIL",
                    f"Missing config sections: {missing_sections}",
                    duration,
                    {"config_content": config_content[:500]}
                )
                return False
            
            self.add_result(
                "Generate Config Command",
                "cli",
                "PASS",
                "Config file generated successfully with expected sections",
                duration,
                {"config_size": len(config_content), "sections_found": len(expected_sections)}
            )
            return True
            
        finally:
            # Clean up temporary file
            if os.path.exists(config_path):
                os.unlink(config_path)
    
    def test_invalid_command(self) -> bool:
        """Test behavior with invalid command"""
        start_time = time.time()
        
        exit_code, stdout, stderr = self.run_command([str(self.binary_path), "invalid-command"])
        duration = time.time() - start_time
        
        # Should fail with non-zero exit code
        if exit_code == 0:
            self.add_result(
                "Invalid Command Handling",
                "cli",
                "FAIL",
                "Invalid command should return non-zero exit code",
                duration,
                {"stdout": stdout, "stderr": stderr}
            )
            return False
        
        # Should provide helpful error message
        error_indicators = ["error", "invalid", "unknown", "help"]
        has_error_info = any(indicator in stderr.lower() or indicator in stdout.lower() 
                           for indicator in error_indicators)
        
        if not has_error_info:
            self.add_result(
                "Invalid Command Error Message",
                "cli",
                "FAIL",
                "Invalid command should provide helpful error message",
                duration,
                {"stdout": stdout, "stderr": stderr}
            )
            return False
        
        self.add_result(
            "Invalid Command Handling",
            "cli",
            "PASS",
            "Invalid command properly rejected with helpful error message",
            duration,
            {"exit_code": exit_code}
        )
        return True
    
    def test_performance_startup(self) -> bool:
        """Test application startup performance"""
        startup_times = []
        
        for i in range(5):  # Run 5 times to get average
            start_time = time.time()
            exit_code, stdout, stderr = self.run_command([str(self.binary_path), "--version"])
            duration = time.time() - start_time
            
            if exit_code == 0:
                startup_times.append(duration)
        
        if not startup_times:
            self.add_result(
                "Startup Performance",
                "performance",
                "FAIL",
                "Could not measure startup time - all runs failed",
                0.0
            )
            return False
        
        avg_startup = sum(startup_times) / len(startup_times)
        max_startup = max(startup_times)
        min_startup = min(startup_times)
        
        # Performance thresholds
        excellent_threshold = 1.0  # < 1 second is excellent
        good_threshold = 3.0      # < 3 seconds is good
        
        if avg_startup < excellent_threshold:
            status = "PASS"
            message = f"Excellent startup performance: {avg_startup:.3f}s average"
        elif avg_startup < good_threshold:
            status = "PASS"
            message = f"Good startup performance: {avg_startup:.3f}s average"
        else:
            status = "FAIL"
            message = f"Slow startup performance: {avg_startup:.3f}s average (threshold: {good_threshold}s)"
        
        self.add_result(
            "Startup Performance",
            "performance",
            status,
            message,
            avg_startup,
            {
                "average_startup": avg_startup,
                "min_startup": min_startup,
                "max_startup": max_startup,
                "runs": len(startup_times)
            }
        )
        return status == "PASS"
    
    def test_memory_usage(self) -> bool:
        """Test memory usage during execution"""
        start_time = time.time()
        
        # This is a simplified memory test - in production you'd use more sophisticated tools
        try:
            import psutil
            
            # Start the process
            process = subprocess.Popen([str(self.binary_path), "supported-formats"], 
                                     stdout=subprocess.PIPE, 
                                     stderr=subprocess.PIPE)
            
            # Monitor memory usage
            memory_samples = []
            try:
                ps_process = psutil.Process(process.pid)
                while process.poll() is None:
                    try:
                        memory_info = ps_process.memory_info()
                        memory_samples.append(memory_info.rss / 1024 / 1024)  # MB
                        time.sleep(0.1)
                    except psutil.NoSuchProcess:
                        break
            except Exception:
                pass
            
            process.wait()
            duration = time.time() - start_time
            
            if memory_samples:
                avg_memory = sum(memory_samples) / len(memory_samples)
                max_memory = max(memory_samples)
                
                # Memory thresholds (MB)
                excellent_threshold = 100  # < 100MB is excellent
                good_threshold = 500      # < 500MB is acceptable
                
                if max_memory < excellent_threshold:
                    status = "PASS"
                    message = f"Excellent memory usage: {max_memory:.1f}MB peak"
                elif max_memory < good_threshold:
                    status = "PASS"
                    message = f"Good memory usage: {max_memory:.1f}MB peak"
                else:
                    status = "FAIL"
                    message = f"High memory usage: {max_memory:.1f}MB peak (threshold: {good_threshold}MB)"
                
                self.add_result(
                    "Memory Usage",
                    "performance",
                    status,
                    message,
                    duration,
                    {
                        "average_memory_mb": avg_memory,
                        "peak_memory_mb": max_memory,
                        "samples": len(memory_samples)
                    }
                )
                return status == "PASS"
            else:
                self.add_result(
                    "Memory Usage",
                    "performance",
                    "SKIP",
                    "Could not collect memory samples",
                    duration
                )
                return True
                
        except ImportError:
            self.add_result(
                "Memory Usage",
                "performance",
                "SKIP",
                "psutil not available for memory monitoring",
                time.time() - start_time
            )
            return True
    
    def test_code_quality(self) -> bool:
        """Test code quality using cargo clippy"""
        start_time = time.time()
        
        exit_code, stdout, stderr = self.run_command(["cargo", "clippy", "--", "-D", "warnings"])
        duration = time.time() - start_time
        
        if exit_code != 0:
            # Count warnings and errors
            warning_count = stderr.count("warning:")
            error_count = stderr.count("error:")
            
            if error_count > 0:
                self.add_result(
                    "Code Quality (Clippy)",
                    "quality",
                    "FAIL",
                    f"Clippy found {error_count} errors and {warning_count} warnings",
                    duration,
                    {"errors": error_count, "warnings": warning_count, "output": stderr[:1000]}
                )
                return False
            else:
                self.add_result(
                    "Code Quality (Clippy)",
                    "quality",
                    "PASS",
                    f"Clippy found {warning_count} warnings but no errors",
                    duration,
                    {"warnings": warning_count}
                )
                return True
        else:
            self.add_result(
                "Code Quality (Clippy)",
                "quality",
                "PASS",
                "Clippy found no issues",
                duration
            )
            return True
    
    def test_build_reproducibility(self) -> bool:
        """Test that the build is reproducible"""
        start_time = time.time()
        
        # Get current binary hash
        import hashlib
        
        if not self.binary_path.exists():
            self.add_result(
                "Build Reproducibility",
                "build",
                "SKIP",
                "Binary not found for hash comparison",
                time.time() - start_time
            )
            return True
        
        with open(self.binary_path, 'rb') as f:
            original_hash = hashlib.sha256(f.read()).hexdigest()
        
        # Rebuild
        exit_code, stdout, stderr = self.run_command(["cargo", "build", "--release"])
        
        if exit_code != 0:
            self.add_result(
                "Build Reproducibility",
                "build",
                "FAIL",
                "Rebuild failed",
                time.time() - start_time,
                {"stderr": stderr[:500]}
            )
            return False
        
        # Check new hash
        with open(self.binary_path, 'rb') as f:
            new_hash = hashlib.sha256(f.read()).hexdigest()
        
        duration = time.time() - start_time
        
        if original_hash == new_hash:
            self.add_result(
                "Build Reproducibility",
                "build",
                "PASS",
                "Build is reproducible - identical binary produced",
                duration,
                {"hash": original_hash[:16] + "..."}
            )
            return True
        else:
            self.add_result(
                "Build Reproducibility",
                "build",
                "FAIL",
                "Build is not reproducible - different binary produced",
                duration,
                {"original_hash": original_hash[:16] + "...", "new_hash": new_hash[:16] + "..."}
            )
            return False
    
    def run_all_tests(self, categories: Optional[List[str]] = None) -> QAReport:
        """Run all QA tests"""
        logger.info("Starting QA test suite for Dattavani ASR Rust Port")
        
        # Define test categories and their tests
        test_categories = {
            "build": [
                self.test_binary_exists,
                self.test_build_reproducibility
            ],
            "cli": [
                self.test_help_command,
                self.test_version_command,
                self.test_supported_formats,
                self.test_generate_config,
                self.test_invalid_command
            ],
            "performance": [
                self.test_performance_startup,
                self.test_memory_usage
            ],
            "quality": [
                self.test_code_quality
            ]
        }
        
        # Filter categories if specified
        if categories:
            test_categories = {k: v for k, v in test_categories.items() if k in categories}
        
        # Run tests
        for category, tests in test_categories.items():
            logger.info(f"Running {category} tests...")
            for test_func in tests:
                try:
                    test_func()
                except Exception as e:
                    self.add_result(
                        test_func.__name__,
                        category,
                        "ERROR",
                        f"Test execution error: {str(e)}",
                        0.0,
                        {"exception": str(e)}
                    )
        
        # Generate report
        total_duration = time.time() - self.start_time
        
        # Count results
        passed = len([r for r in self.results if r.status == "PASS"])
        failed = len([r for r in self.results if r.status == "FAIL"])
        skipped = len([r for r in self.results if r.status == "SKIP"])
        errors = len([r for r in self.results if r.status == "ERROR"])
        
        # Create summary
        summary = {
            "overall_status": "PASS" if failed == 0 and errors == 0 else "FAIL",
            "pass_rate": passed / len(self.results) if self.results else 0,
            "categories_tested": list(test_categories.keys()),
            "binary_path": str(self.binary_path),
            "binary_exists": self.binary_path.exists(),
            "binary_size_mb": self.binary_path.stat().st_size / 1024 / 1024 if self.binary_path.exists() else 0
        }
        
        report = QAReport(
            timestamp=datetime.now().isoformat(),
            total_tests=len(self.results),
            passed=passed,
            failed=failed,
            skipped=skipped,
            errors=errors,
            duration=total_duration,
            results=self.results,
            summary=summary
        )
        
        logger.info(f"QA test suite completed: {passed} passed, {failed} failed, {skipped} skipped, {errors} errors")
        return report
    
    def save_report(self, report: QAReport, format: str = "json") -> str:
        """Save QA report to file"""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        
        if format == "json":
            report_path = self.qa_root / "reports" / f"qa_report_{timestamp}.json"
            with open(report_path, 'w') as f:
                json.dump(asdict(report), f, indent=2, default=str)
        
        elif format == "html":
            report_path = self.qa_root / "reports" / f"qa_report_{timestamp}.html"
            html_content = self.generate_html_report(report)
            with open(report_path, 'w') as f:
                f.write(html_content)
        
        logger.info(f"QA report saved to {report_path}")
        return str(report_path)
    
    def generate_html_report(self, report: QAReport) -> str:
        """Generate HTML report"""
        html = f"""
<!DOCTYPE html>
<html>
<head>
    <title>Dattavani ASR QA Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .summary {{ margin: 20px 0; }}
        .test-result {{ margin: 10px 0; padding: 10px; border-radius: 3px; }}
        .pass {{ background: #d4edda; border-left: 4px solid #28a745; }}
        .fail {{ background: #f8d7da; border-left: 4px solid #dc3545; }}
        .skip {{ background: #fff3cd; border-left: 4px solid #ffc107; }}
        .error {{ background: #f5c6cb; border-left: 4px solid #dc3545; }}
        .details {{ font-size: 0.9em; color: #666; margin-top: 5px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Dattavani ASR QA Report</h1>
        <p><strong>Generated:</strong> {report.timestamp}</p>
        <p><strong>Duration:</strong> {report.duration:.2f} seconds</p>
        <p><strong>Overall Status:</strong> {report.summary['overall_status']}</p>
    </div>
    
    <div class="summary">
        <h2>Summary</h2>
        <ul>
            <li><strong>Total Tests:</strong> {report.total_tests}</li>
            <li><strong>Passed:</strong> {report.passed}</li>
            <li><strong>Failed:</strong> {report.failed}</li>
            <li><strong>Skipped:</strong> {report.skipped}</li>
            <li><strong>Errors:</strong> {report.errors}</li>
            <li><strong>Pass Rate:</strong> {report.summary['pass_rate']:.1%}</li>
        </ul>
    </div>
    
    <div class="results">
        <h2>Test Results</h2>
"""
        
        for result in report.results:
            status_class = result.status.lower()
            html += f"""
        <div class="test-result {status_class}">
            <strong>{result.name}</strong> ({result.category})
            <span style="float: right;">{result.status} - {result.duration:.3f}s</span>
            <div class="details">{result.message}</div>
        </div>
"""
        
        html += """
    </div>
</body>
</html>
"""
        return html

def main():
    """Main entry point for QA agent"""
    parser = argparse.ArgumentParser(description="QA Agent for Dattavani ASR Rust Port")
    parser.add_argument("--project-root", default=".", help="Project root directory")
    parser.add_argument("--categories", nargs="+", 
                       choices=["build", "cli", "performance", "quality"],
                       help="Test categories to run")
    parser.add_argument("--format", choices=["json", "html"], default="json",
                       help="Report format")
    parser.add_argument("--output", help="Output file path")
    
    args = parser.parse_args()
    
    # Initialize QA agent
    qa_agent = DattavaniQAAgent(args.project_root)
    
    # Run tests
    report = qa_agent.run_all_tests(args.categories)
    
    # Save report
    if args.output:
        report_path = args.output
        if args.format == "json":
            with open(report_path, 'w') as f:
                json.dump(asdict(report), f, indent=2, default=str)
        elif args.format == "html":
            with open(report_path, 'w') as f:
                f.write(qa_agent.generate_html_report(report))
    else:
        report_path = qa_agent.save_report(report, args.format)
    
    # Print summary
    print(f"\n{'='*60}")
    print(f"QA REPORT SUMMARY")
    print(f"{'='*60}")
    print(f"Overall Status: {report.summary['overall_status']}")
    print(f"Tests Run: {report.total_tests}")
    print(f"Passed: {report.passed}")
    print(f"Failed: {report.failed}")
    print(f"Skipped: {report.skipped}")
    print(f"Errors: {report.errors}")
    print(f"Pass Rate: {report.summary['pass_rate']:.1%}")
    print(f"Duration: {report.duration:.2f} seconds")
    print(f"Report saved to: {report_path}")
    print(f"{'='*60}")
    
    # Exit with appropriate code
    sys.exit(0 if report.summary['overall_status'] == 'PASS' else 1)

if __name__ == "__main__":
    main()
