#!/usr/bin/env python3
"""
Performance Benchmark Script for CI/CD

Runs comprehensive performance benchmarks on the Dattavani ASR binary.
"""

import json
import subprocess
import time
import sys
import os
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any
import tempfile

class PerformanceBenchmark:
    """Performance benchmarking for Dattavani ASR"""
    
    def __init__(self, project_root: str = "."):
        self.project_root = Path(project_root)
        self.binary_path = self.project_root / "target" / "release" / "dattavani-asr"
        
        if not self.binary_path.exists():
            raise FileNotFoundError(f"Binary not found at {self.binary_path}")
    
    def run_command(self, cmd: List[str], timeout: int = 30) -> tuple:
        """Run a command and return exit code, stdout, stderr, duration"""
        start_time = time.time()
        try:
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
            duration = time.time() - start_time
            return result.returncode, result.stdout, result.stderr, duration
        except subprocess.TimeoutExpired:
            duration = time.time() - start_time
            return -1, "", "Command timed out", duration
        except Exception as e:
            duration = time.time() - start_time
            return -1, "", str(e), duration
    
    def benchmark_startup_time(self, runs: int = 10) -> Dict[str, Any]:
        """Benchmark application startup time"""
        startup_times = []
        
        for i in range(runs):
            exit_code, stdout, stderr, duration = self.run_command([str(self.binary_path), "--version"])
            
            if exit_code == 0:
                startup_times.append(duration)
        
        if not startup_times:
            return {
                "status": "FAIL",
                "message": "Could not measure startup time - all runs failed"
            }
        
        avg_time = sum(startup_times) / len(startup_times)
        min_time = min(startup_times)
        max_time = max(startup_times)
        
        # Performance rating
        if avg_time < 0.1:
            rating = "EXCELLENT"
        elif avg_time < 0.5:
            rating = "VERY_GOOD"
        elif avg_time < 1.0:
            rating = "GOOD"
        elif avg_time < 3.0:
            rating = "ACCEPTABLE"
        else:
            rating = "POOR"
        
        return {
            "status": "PASS",
            "metric": "startup_time",
            "runs": len(startup_times),
            "average_seconds": avg_time,
            "min_seconds": min_time,
            "max_seconds": max_time,
            "std_deviation": (sum((t - avg_time) ** 2 for t in startup_times) / len(startup_times)) ** 0.5,
            "rating": rating,
            "message": f"Average startup time: {avg_time:.3f}s ({rating})"
        }
    
    def benchmark_help_command(self, runs: int = 5) -> Dict[str, Any]:
        """Benchmark help command performance"""
        help_times = []
        
        for i in range(runs):
            exit_code, stdout, stderr, duration = self.run_command([str(self.binary_path), "--help"])
            
            if exit_code == 0:
                help_times.append(duration)
        
        if not help_times:
            return {
                "status": "FAIL",
                "message": "Could not measure help command time"
            }
        
        avg_time = sum(help_times) / len(help_times)
        
        return {
            "status": "PASS",
            "metric": "help_command_time",
            "runs": len(help_times),
            "average_seconds": avg_time,
            "message": f"Average help command time: {avg_time:.3f}s"
        }
    
    def benchmark_config_generation(self, runs: int = 3) -> Dict[str, Any]:
        """Benchmark config generation performance"""
        config_times = []
        
        for i in range(runs):
            with tempfile.NamedTemporaryFile(suffix=".toml", delete=False) as tmp_file:
                config_path = tmp_file.name
            
            try:
                exit_code, stdout, stderr, duration = self.run_command([
                    str(self.binary_path), 
                    "generate-config", 
                    "--output", 
                    config_path
                ])
                
                if exit_code == 0 and os.path.exists(config_path):
                    config_times.append(duration)
                
            finally:
                if os.path.exists(config_path):
                    os.unlink(config_path)
        
        if not config_times:
            return {
                "status": "FAIL",
                "message": "Could not measure config generation time"
            }
        
        avg_time = sum(config_times) / len(config_times)
        
        return {
            "status": "PASS",
            "metric": "config_generation_time",
            "runs": len(config_times),
            "average_seconds": avg_time,
            "message": f"Average config generation time: {avg_time:.3f}s"
        }
    
    def benchmark_binary_size(self) -> Dict[str, Any]:
        """Analyze binary size and characteristics"""
        try:
            stat = self.binary_path.stat()
            size_bytes = stat.st_size
            size_mb = size_bytes / (1024 * 1024)
            
            # Size rating
            if size_mb < 5:
                rating = "EXCELLENT"
            elif size_mb < 10:
                rating = "VERY_GOOD"
            elif size_mb < 20:
                rating = "GOOD"
            elif size_mb < 50:
                rating = "ACCEPTABLE"
            else:
                rating = "LARGE"
            
            return {
                "status": "PASS",
                "metric": "binary_size",
                "size_bytes": size_bytes,
                "size_mb": size_mb,
                "rating": rating,
                "message": f"Binary size: {size_mb:.1f} MB ({rating})"
            }
            
        except Exception as e:
            return {
                "status": "ERROR",
                "message": f"Could not analyze binary size: {str(e)}"
            }
    
    def benchmark_memory_usage(self) -> Dict[str, Any]:
        """Benchmark memory usage (requires psutil)"""
        try:
            import psutil
        except ImportError:
            return {
                "status": "SKIP",
                "message": "psutil not available for memory benchmarking"
            }
        
        try:
            # Start the process
            process = subprocess.Popen([str(self.binary_path), "supported-formats"], 
                                     stdout=subprocess.PIPE, 
                                     stderr=subprocess.PIPE)
            
            # Monitor memory usage
            memory_samples = []
            try:
                ps_process = psutil.Process(process.pid)
                start_time = time.time()
                
                while process.poll() is None and time.time() - start_time < 10:
                    try:
                        memory_info = ps_process.memory_info()
                        memory_samples.append(memory_info.rss / 1024 / 1024)  # MB
                        time.sleep(0.1)
                    except psutil.NoSuchProcess:
                        break
            except Exception:
                pass
            
            process.wait()
            
            if memory_samples:
                avg_memory = sum(memory_samples) / len(memory_samples)
                max_memory = max(memory_samples)
                
                # Memory rating
                if max_memory < 50:
                    rating = "EXCELLENT"
                elif max_memory < 100:
                    rating = "VERY_GOOD"
                elif max_memory < 200:
                    rating = "GOOD"
                elif max_memory < 500:
                    rating = "ACCEPTABLE"
                else:
                    rating = "HIGH"
                
                return {
                    "status": "PASS",
                    "metric": "memory_usage",
                    "average_mb": avg_memory,
                    "peak_mb": max_memory,
                    "samples": len(memory_samples),
                    "rating": rating,
                    "message": f"Peak memory usage: {max_memory:.1f} MB ({rating})"
                }
            else:
                return {
                    "status": "FAIL",
                    "message": "Could not collect memory samples"
                }
                
        except Exception as e:
            return {
                "status": "ERROR",
                "message": f"Memory benchmarking failed: {str(e)}"
            }
    
    def benchmark_concurrent_commands(self, concurrent: int = 5) -> Dict[str, Any]:
        """Benchmark concurrent command execution"""
        import threading
        
        results = []
        threads = []
        
        def run_version_command():
            exit_code, stdout, stderr, duration = self.run_command([str(self.binary_path), "--version"])
            results.append({
                "exit_code": exit_code,
                "duration": duration,
                "success": exit_code == 0
            })
        
        # Start concurrent threads
        start_time = time.time()
        for i in range(concurrent):
            thread = threading.Thread(target=run_version_command)
            threads.append(thread)
            thread.start()
        
        # Wait for all threads
        for thread in threads:
            thread.join()
        
        total_time = time.time() - start_time
        
        successful_runs = [r for r in results if r["success"]]
        
        if not successful_runs:
            return {
                "status": "FAIL",
                "message": "No concurrent commands succeeded"
            }
        
        avg_duration = sum(r["duration"] for r in successful_runs) / len(successful_runs)
        
        return {
            "status": "PASS",
            "metric": "concurrent_execution",
            "concurrent_processes": concurrent,
            "successful_runs": len(successful_runs),
            "total_time": total_time,
            "average_command_duration": avg_duration,
            "message": f"Concurrent execution: {len(successful_runs)}/{concurrent} succeeded"
        }
    
    def run_all_benchmarks(self) -> Dict[str, Any]:
        """Run all performance benchmarks"""
        benchmarks = {
            "startup_time": self.benchmark_startup_time(),
            "help_command": self.benchmark_help_command(),
            "config_generation": self.benchmark_config_generation(),
            "binary_size": self.benchmark_binary_size(),
            "memory_usage": self.benchmark_memory_usage(),
            "concurrent_execution": self.benchmark_concurrent_commands()
        }
        
        # Calculate overall performance score
        ratings = []
        for benchmark in benchmarks.values():
            if benchmark["status"] == "PASS" and "rating" in benchmark:
                rating_scores = {
                    "EXCELLENT": 5,
                    "VERY_GOOD": 4,
                    "GOOD": 3,
                    "ACCEPTABLE": 2,
                    "POOR": 1,
                    "HIGH": 1,
                    "LARGE": 1
                }
                ratings.append(rating_scores.get(benchmark["rating"], 3))
        
        overall_score = sum(ratings) / len(ratings) if ratings else 3
        
        if overall_score >= 4.5:
            overall_rating = "EXCELLENT"
        elif overall_score >= 3.5:
            overall_rating = "VERY_GOOD"
        elif overall_score >= 2.5:
            overall_rating = "GOOD"
        elif overall_score >= 1.5:
            overall_rating = "ACCEPTABLE"
        else:
            overall_rating = "POOR"
        
        return {
            "timestamp": datetime.now().isoformat(),
            "binary_path": str(self.binary_path),
            "overall_rating": overall_rating,
            "overall_score": overall_score,
            "benchmarks": benchmarks,
            "summary": {
                "startup_time": benchmarks["startup_time"].get("average_seconds", 0),
                "binary_size_mb": benchmarks["binary_size"].get("size_mb", 0),
                "peak_memory_mb": benchmarks["memory_usage"].get("peak_mb", 0),
                "concurrent_success_rate": benchmarks["concurrent_execution"].get("successful_runs", 0) / 5
            }
        }

def main():
    """Main entry point"""
    project_root = sys.argv[1] if len(sys.argv) > 1 else "."
    
    try:
        benchmark = PerformanceBenchmark(project_root)
        results = benchmark.run_all_benchmarks()
        
        print(json.dumps(results, indent=2))
        
        # Exit with appropriate code based on performance
        exit_code = 0 if results["overall_rating"] in ["EXCELLENT", "VERY_GOOD", "GOOD"] else 1
        sys.exit(exit_code)
        
    except Exception as e:
        error_result = {
            "timestamp": datetime.now().isoformat(),
            "status": "ERROR",
            "message": f"Benchmark failed: {str(e)}"
        }
        print(json.dumps(error_result, indent=2))
        sys.exit(1)

if __name__ == "__main__":
    main()
