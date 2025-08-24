# QA Agent for Dattavani ASR Rust Port

This QA agent performs comprehensive testing and validation of the Dattavani ASR Rust application to ensure quality, functionality, and performance standards are met.

## Features

- **Automated Testing**: Unit, integration, and end-to-end tests
- **Performance Validation**: Benchmarking and performance regression detection
- **Code Quality**: Static analysis, linting, and security checks
- **Functional Verification**: CLI commands, API endpoints, and feature validation
- **Regression Testing**: Comparison with previous versions and Python baseline
- **Report Generation**: Detailed QA reports with pass/fail status

## Usage

```bash
# Run full QA suite
./qa-agent/run_qa.sh

# Run specific test categories
./qa-agent/run_qa.sh --tests unit
./qa-agent/run_qa.sh --tests integration
./qa-agent/run_qa.sh --tests performance

# Generate QA report
./qa-agent/generate_report.sh
```
