#!/bin/bash

# Comprehensive debugging script for Dattavani ASR
AUDIO_FILE="${1:-test_audio.wav}"
OUTPUT_FILE="debug_session_$(date +%Y%m%d_%H%M%S).log"
TIMEOUT_DURATION=120

echo "Dattavani ASR Debug Session"
echo "Audio file: $AUDIO_FILE"
echo "Output file: $OUTPUT_FILE"
echo "Timeout: ${TIMEOUT_DURATION}s"
echo "================================"

# Initialize log file
cat > "$OUTPUT_FILE" << EOF
Dattavani ASR Debug Session - Started at $(date)
Audio File: $AUDIO_FILE
Timeout: ${TIMEOUT_DURATION}s
========================================

EOF

# Function to log system state
log_system_state() {
    echo "=== System State - $(date) ===" >> "$OUTPUT_FILE"
    
    # Process info
    echo "Active dattavani/whisper processes:" >> "$OUTPUT_FILE"
    ps aux | grep -E "(dattavani|whisper)" | grep -v grep >> "$OUTPUT_FILE" 2>/dev/null || echo "No processes found" >> "$OUTPUT_FILE"
    
    # System resources
    echo "" >> "$OUTPUT_FILE"
    echo "CPU/Memory:" >> "$OUTPUT_FILE"
    top -l 1 -n 0 | grep -E "(CPU usage|PhysMem)" >> "$OUTPUT_FILE"
    
    # GPU state
    echo "" >> "$OUTPUT_FILE"
    echo "GPU state:" >> "$OUTPUT_FILE"
    sudo powermetrics -n 1 -i 500 --samplers gpu_power 2>/dev/null | grep -E "(GPU HW active|GPU Power|GPU idle)" >> "$OUTPUT_FILE" 2>/dev/null || echo "GPU monitoring not available" >> "$OUTPUT_FILE"
    
    # Temp directory
    echo "" >> "$OUTPUT_FILE"
    echo "Temp directory contents:" >> "$OUTPUT_FILE"
    ls -la /var/folders/91/g5z_zd1d2ns39m1b6lcn9bnw0000gn/T/dattavani_asr/ 2>/dev/null | tail -5 >> "$OUTPUT_FILE" || echo "Temp dir not accessible" >> "$OUTPUT_FILE"
    
    echo "" >> "$OUTPUT_FILE"
}

# Function to monitor process
monitor_process() {
    local pid=$1
    echo "=== Process Monitor - PID $pid - $(date) ===" >> "$OUTPUT_FILE"
    
    if ps -p $pid > /dev/null 2>&1; then
        ps -p $pid -o pid,ppid,user,%cpu,%mem,vsz,rss,stat,start,time,command >> "$OUTPUT_FILE"
        
        # Check what files the process is accessing
        echo "" >> "$OUTPUT_FILE"
        echo "Open files:" >> "$OUTPUT_FILE"
        lsof -p $pid 2>/dev/null | grep -v "REG.*/" | head -10 >> "$OUTPUT_FILE" || echo "lsof not available" >> "$OUTPUT_FILE"
        
        # Check system calls (if available)
        echo "" >> "$OUTPUT_FILE"
        echo "Process state:" >> "$OUTPUT_FILE"
        ps -p $pid -o pid,stat,wchan >> "$OUTPUT_FILE"
        
    else
        echo "Process $pid not found" >> "$OUTPUT_FILE"
        return 1
    fi
    
    echo "" >> "$OUTPUT_FILE"
    return 0
}

# Start monitoring in background
log_system_state

echo "Starting application with monitoring..."
echo "--- Application Start - $(date) ---" >> "$OUTPUT_FILE"

# Start the application in background
timeout $TIMEOUT_DURATION ./target/release/dattavani-asr stream-process "$AUDIO_FILE" --language en > app_output.log 2>&1 &
APP_PID=$!

echo "Application started with PID: $APP_PID"
echo "Application PID: $APP_PID" >> "$OUTPUT_FILE"

# Monitor the application
MONITOR_COUNT=0
while kill -0 $APP_PID 2>/dev/null; do
    MONITOR_COUNT=$((MONITOR_COUNT + 1))
    echo "Monitor cycle $MONITOR_COUNT..."
    
    monitor_process $APP_PID
    log_system_state
    
    # Check for child processes
    echo "=== Child Processes - $(date) ===" >> "$OUTPUT_FILE"
    pgrep -P $APP_PID >> "$OUTPUT_FILE" 2>/dev/null || echo "No child processes" >> "$OUTPUT_FILE"
    
    # Show recent application output
    echo "=== Recent App Output ===" >> "$OUTPUT_FILE"
    tail -5 app_output.log >> "$OUTPUT_FILE" 2>/dev/null || echo "No output yet" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    
    sleep 10
    
    # Safety check - if monitoring too long, break
    if [ $MONITOR_COUNT -gt 20 ]; then
        echo "Monitoring limit reached, stopping..."
        break
    fi
done

# Wait for application to finish
wait $APP_PID 2>/dev/null
EXIT_CODE=$?

echo "--- Application End - $(date) ---" >> "$OUTPUT_FILE"
echo "Exit code: $EXIT_CODE" >> "$OUTPUT_FILE"

# Final system state
log_system_state

# Show final application output
echo "=== Final Application Output ===" >> "$OUTPUT_FILE"
cat app_output.log >> "$OUTPUT_FILE" 2>/dev/null || echo "No application output" >> "$OUTPUT_FILE"

echo ""
echo "Debug session completed!"
echo "Exit code: $EXIT_CODE"
echo "Results saved to: $OUTPUT_FILE"
echo ""
echo "Final application output:"
cat app_output.log 2>/dev/null || echo "No output file found"

# Cleanup
rm -f app_output.log
