#!/usr/bin/env python3
"""
Security Check Script for CI/CD

Performs security analysis on the Dattavani ASR codebase.
"""

import json
import subprocess
import sys
import os
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any

class SecurityChecker:
    """Security analysis for Rust projects"""
    
    def __init__(self, project_root: str = "."):
        self.project_root = Path(project_root)
        self.results = []
    
    def run_command(self, cmd: List[str]) -> tuple:
        """Run a command and return exit code, stdout, stderr"""
        try:
            result = subprocess.run(cmd, capture_output=True, text=True, cwd=self.project_root)
            return result.returncode, result.stdout, result.stderr
        except Exception as e:
            return -1, "", str(e)
    
    def check_cargo_audit(self) -> Dict[str, Any]:
        """Run cargo audit for known vulnerabilities"""
        exit_code, stdout, stderr = self.run_command(["cargo", "audit", "--json"])
        
        if exit_code == 0:
            try:
                audit_data = json.loads(stdout)
                vulnerabilities = audit_data.get('vulnerabilities', {}).get('list', [])
                
                return {
                    "status": "PASS" if len(vulnerabilities) == 0 else "FAIL",
                    "vulnerabilities_found": len(vulnerabilities),
                    "vulnerabilities": vulnerabilities[:5],  # Limit to first 5
                    "message": f"Found {len(vulnerabilities)} known vulnerabilities"
                }
            except json.JSONDecodeError:
                return {
                    "status": "ERROR",
                    "message": "Could not parse cargo audit output",
                    "raw_output": stdout[:500]
                }
        else:
            return {
                "status": "ERROR",
                "message": f"cargo audit failed with exit code {exit_code}",
                "error": stderr[:500]
            }
    
    def check_dependencies(self) -> Dict[str, Any]:
        """Check for suspicious or outdated dependencies"""
        cargo_toml = self.project_root / "Cargo.toml"
        
        if not cargo_toml.exists():
            return {
                "status": "ERROR",
                "message": "Cargo.toml not found"
            }
        
        # Read Cargo.toml
        try:
            with open(cargo_toml, 'r') as f:
                content = f.read()
            
            # Simple checks
            issues = []
            
            # Check for git dependencies (potential security risk)
            if "git =" in content:
                issues.append("Git dependencies found (potential security risk)")
            
            # Check for path dependencies outside project
            if "path =" in content and ".." in content:
                issues.append("External path dependencies found")
            
            # Check for wildcard versions
            if '"*"' in content:
                issues.append("Wildcard version dependencies found")
            
            return {
                "status": "PASS" if len(issues) == 0 else "WARN",
                "issues_found": len(issues),
                "issues": issues,
                "message": f"Found {len(issues)} dependency issues"
            }
            
        except Exception as e:
            return {
                "status": "ERROR",
                "message": f"Could not analyze dependencies: {str(e)}"
            }
    
    def check_secrets(self) -> Dict[str, Any]:
        """Check for potential secrets in code"""
        secrets_patterns = [
            r"password\s*=\s*['\"][^'\"]+['\"]",
            r"api_key\s*=\s*['\"][^'\"]+['\"]",
            r"secret\s*=\s*['\"][^'\"]+['\"]",
            r"token\s*=\s*['\"][^'\"]+['\"]",
            r"-----BEGIN.*PRIVATE KEY-----",
        ]
        
        import re
        
        secrets_found = []
        
        # Check Rust source files
        for rust_file in self.project_root.rglob("*.rs"):
            try:
                with open(rust_file, 'r') as f:
                    content = f.read()
                
                for pattern in secrets_patterns:
                    matches = re.findall(pattern, content, re.IGNORECASE)
                    for match in matches:
                        secrets_found.append({
                            "file": str(rust_file.relative_to(self.project_root)),
                            "pattern": pattern,
                            "match": match[:50] + "..." if len(match) > 50 else match
                        })
            except Exception:
                continue
        
        return {
            "status": "PASS" if len(secrets_found) == 0 else "FAIL",
            "secrets_found": len(secrets_found),
            "secrets": secrets_found[:5],  # Limit to first 5
            "message": f"Found {len(secrets_found)} potential secrets"
        }
    
    def check_unsafe_code(self) -> Dict[str, Any]:
        """Check for unsafe Rust code blocks"""
        unsafe_blocks = []
        
        for rust_file in self.project_root.rglob("*.rs"):
            try:
                with open(rust_file, 'r') as f:
                    lines = f.readlines()
                
                for i, line in enumerate(lines):
                    if "unsafe" in line and "{" in line:
                        unsafe_blocks.append({
                            "file": str(rust_file.relative_to(self.project_root)),
                            "line": i + 1,
                            "code": line.strip()
                        })
            except Exception:
                continue
        
        return {
            "status": "PASS" if len(unsafe_blocks) == 0 else "WARN",
            "unsafe_blocks_found": len(unsafe_blocks),
            "unsafe_blocks": unsafe_blocks[:5],
            "message": f"Found {len(unsafe_blocks)} unsafe code blocks"
        }
    
    def check_file_permissions(self) -> Dict[str, Any]:
        """Check for files with overly permissive permissions"""
        if os.name == 'nt':  # Windows
            return {
                "status": "SKIP",
                "message": "File permission check skipped on Windows"
            }
        
        suspicious_files = []
        
        for file_path in self.project_root.rglob("*"):
            if file_path.is_file():
                try:
                    stat = file_path.stat()
                    mode = oct(stat.st_mode)[-3:]
                    
                    # Check for world-writable files
                    if mode.endswith('6') or mode.endswith('7'):
                        suspicious_files.append({
                            "file": str(file_path.relative_to(self.project_root)),
                            "permissions": mode,
                            "issue": "World-writable file"
                        })
                except Exception:
                    continue
        
        return {
            "status": "PASS" if len(suspicious_files) == 0 else "WARN",
            "suspicious_files_found": len(suspicious_files),
            "suspicious_files": suspicious_files[:5],
            "message": f"Found {len(suspicious_files)} files with suspicious permissions"
        }
    
    def run_all_checks(self) -> Dict[str, Any]:
        """Run all security checks"""
        checks = {
            "cargo_audit": self.check_cargo_audit(),
            "dependencies": self.check_dependencies(),
            "secrets": self.check_secrets(),
            "unsafe_code": self.check_unsafe_code(),
            "file_permissions": self.check_file_permissions()
        }
        
        # Determine overall status
        statuses = [check["status"] for check in checks.values()]
        
        if "FAIL" in statuses:
            overall_status = "FAIL"
        elif "ERROR" in statuses:
            overall_status = "ERROR"
        elif "WARN" in statuses:
            overall_status = "WARN"
        else:
            overall_status = "PASS"
        
        # Count issues
        total_issues = 0
        critical_issues = 0
        
        for check in checks.values():
            if check["status"] == "FAIL":
                critical_issues += 1
            if check["status"] in ["FAIL", "WARN"]:
                total_issues += 1
        
        return {
            "timestamp": datetime.now().isoformat(),
            "overall_status": overall_status,
            "total_checks": len(checks),
            "total_issues": total_issues,
            "critical_issues": critical_issues,
            "checks": checks,
            "summary": {
                "vulnerabilities": checks["cargo_audit"].get("vulnerabilities_found", 0),
                "secrets": checks["secrets"].get("secrets_found", 0),
                "unsafe_blocks": checks["unsafe_code"].get("unsafe_blocks_found", 0),
                "dependency_issues": checks["dependencies"].get("issues_found", 0),
                "permission_issues": checks["file_permissions"].get("suspicious_files_found", 0)
            }
        }

def main():
    """Main entry point"""
    project_root = sys.argv[1] if len(sys.argv) > 1 else "."
    
    checker = SecurityChecker(project_root)
    results = checker.run_all_checks()
    
    print(json.dumps(results, indent=2))
    
    # Exit with appropriate code
    exit_code = 0 if results["overall_status"] in ["PASS", "WARN"] else 1
    sys.exit(exit_code)

if __name__ == "__main__":
    main()
