#!/bin/bash

# QA Agent Runner Script for Dattavani ASR Rust Port
# This script runs the comprehensive QA test suite

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${BLUE}🔍 Dattavani ASR QA Agent${NC}"
echo -e "${BLUE}=========================${NC}"
echo "Project Root: $PROJECT_ROOT"
echo "QA Agent: $SCRIPT_DIR"
echo ""

# Check if Python 3 is available
if ! command -v python3 &> /dev/null; then
    echo -e "${RED}❌ Python 3 is required but not installed${NC}"
    exit 1
fi

# Check if the binary exists
BINARY_PATH="$PROJECT_ROOT/target/release/dattavani-asr"
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${YELLOW}⚠️  Binary not found at $BINARY_PATH${NC}"
    echo -e "${YELLOW}   Building release binary...${NC}"
    cd "$PROJECT_ROOT"
    cargo build --release
    if [ $? -ne 0 ]; then
        echo -e "${RED}❌ Failed to build release binary${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Binary built successfully${NC}"
fi

# Install Python dependencies if needed
REQUIREMENTS_FILE="$SCRIPT_DIR/requirements.txt"
if [ -f "$REQUIREMENTS_FILE" ]; then
    echo -e "${BLUE}📦 Installing Python dependencies...${NC}"
    pip3 install -r "$REQUIREMENTS_FILE" --quiet
fi

# Create reports directory
mkdir -p "$SCRIPT_DIR/reports"

# Parse command line arguments
CATEGORIES=""
FORMAT="json"
OUTPUT=""
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --categories|--tests)
            shift
            CATEGORIES="$1"
            ;;
        --format)
            shift
            FORMAT="$1"
            ;;
        --output)
            shift
            OUTPUT="$1"
            ;;
        --verbose|-v)
            VERBOSE=true
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --categories CATS    Test categories to run (build,cli,performance,quality)"
            echo "  --format FORMAT      Report format (json,html)"
            echo "  --output FILE        Output file path"
            echo "  --verbose, -v        Verbose output"
            echo "  --help, -h           Show this help"
            echo ""
            echo "Examples:"
            echo "  $0                           # Run all tests"
            echo "  $0 --categories cli          # Run only CLI tests"
            echo "  $0 --format html             # Generate HTML report"
            echo "  $0 --output report.json      # Save to specific file"
            exit 0
            ;;
        *)
            echo -e "${RED}❌ Unknown option: $1${NC}"
            exit 1
            ;;
    esac
    shift
done

# Build QA agent command
QA_CMD="python3 $SCRIPT_DIR/qa_agent.py --project-root $PROJECT_ROOT"

if [ -n "$CATEGORIES" ]; then
    QA_CMD="$QA_CMD --categories $CATEGORIES"
fi

if [ -n "$FORMAT" ]; then
    QA_CMD="$QA_CMD --format $FORMAT"
fi

if [ -n "$OUTPUT" ]; then
    QA_CMD="$QA_CMD --output $OUTPUT"
fi

# Run pre-checks
echo -e "${BLUE}🔧 Running pre-checks...${NC}"

# Check Rust toolchain
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Cargo (Rust) is required but not installed${NC}"
    exit 1
fi

# Check binary size and permissions
if [ -f "$BINARY_PATH" ]; then
    BINARY_SIZE=$(du -h "$BINARY_PATH" | cut -f1)
    echo -e "${GREEN}✅ Binary found: $BINARY_SIZE${NC}"
    
    if [ ! -x "$BINARY_PATH" ]; then
        echo -e "${YELLOW}⚠️  Making binary executable...${NC}"
        chmod +x "$BINARY_PATH"
    fi
else
    echo -e "${RED}❌ Binary not found after build attempt${NC}"
    exit 1
fi

# Run QA tests
echo -e "${BLUE}🧪 Running QA test suite...${NC}"
echo "Command: $QA_CMD"
echo ""

if [ "$VERBOSE" = true ]; then
    $QA_CMD
    QA_EXIT_CODE=$?
else
    $QA_CMD 2>/dev/null
    QA_EXIT_CODE=$?
fi

# Report results
echo ""
if [ $QA_EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}🎉 QA tests completed successfully!${NC}"
    
    # Show latest report if it exists
    LATEST_REPORT=$(ls -t "$SCRIPT_DIR/reports"/qa_report_*.json 2>/dev/null | head -1)
    if [ -n "$LATEST_REPORT" ]; then
        echo -e "${BLUE}📊 Latest report: $LATEST_REPORT${NC}"
        
        # Extract summary from JSON report
        if command -v jq &> /dev/null; then
            echo -e "${BLUE}📈 Quick Summary:${NC}"
            jq -r '"  Total Tests: " + (.total_tests | tostring) + 
                   "\n  Passed: " + (.passed | tostring) + 
                   "\n  Failed: " + (.failed | tostring) + 
                   "\n  Pass Rate: " + ((.summary.pass_rate * 100) | floor | tostring) + "%"' "$LATEST_REPORT"
        fi
    fi
else
    echo -e "${RED}❌ QA tests failed!${NC}"
    echo -e "${YELLOW}📋 Check the detailed report for more information${NC}"
fi

# Cleanup
echo -e "${BLUE}🧹 Cleaning up temporary files...${NC}"
# Add any cleanup commands here

echo -e "${BLUE}✨ QA run completed${NC}"
exit $QA_EXIT_CODE
