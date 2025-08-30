#!/bin/bash

# Process monitoring script
PID=${1:-31572}
OUTPUT_FILE="process_monitor_$(date +%Y%m%d_%H%M%S).log"

echo "Process Monitoring - PID: $PID"
echo "Output file: $OUTPUT_FILE"
echo "================================"

# Initialize log file
cat > "$OUTPUT_FILE" << EOF
Process Monitoring - Started at $(date)
Target PID: $PID
========================================

EOF

# Function to get process info
get_process_info() {
    echo "=== Process Info - $(date) ===" >> "$OUTPUT_FILE"
    
    if ps -p $PID > /dev/null 2>&1; then
        ps -p $PID -o pid,ppid,user,%cpu,%mem,vsz,rss,tty,stat,start,time,command >> "$OUTPUT_FILE"
        
        # Get detailed process info
        echo "" >> "$OUTPUT_FILE"
        echo "Process details:" >> "$OUTPUT_FILE"
        ps -p $PID -o pid,ppid,user,nice,pri,vsz,rss,wchan,stat,tt,time,command >> "$OUTPUT_FILE"
        
        # Check open files
        echo "" >> "$OUTPUT_FILE"
        echo "Open files (sample):" >> "$OUTPUT_FILE"
        lsof -p $PID 2>/dev/null | head -10 >> "$OUTPUT_FILE" || echo "lsof not available" >> "$OUTPUT_FILE"
        
        # Check network connections
        echo "" >> "$OUTPUT_FILE"
        echo "Network connections:" >> "$OUTPUT_FILE"
        lsof -i -p $PID 2>/dev/null >> "$OUTPUT_FILE" || echo "No network connections" >> "$OUTPUT_FILE"
        
    else
        echo "Process $PID not found or has terminated" >> "$OUTPUT_FILE"
        return 1
    fi
    
    echo "" >> "$OUTPUT_FILE"
    return 0
}

# Function to get system resource usage
get_system_info() {
    echo "=== System Resources - $(date) ===" >> "$OUTPUT_FILE"
    
    # CPU usage
    top -l 1 -n 0 | grep "CPU usage" >> "$OUTPUT_FILE"
    
    # Memory usage
    vm_stat | head -5 >> "$OUTPUT_FILE"
    
    # GPU usage
    sudo powermetrics -n 1 -i 500 --samplers gpu_power 2>/dev/null | grep -E "(GPU HW active|GPU Power|GPU idle)" >> "$OUTPUT_FILE" 2>/dev/null || echo "GPU stats not available" >> "$OUTPUT_FILE"
    
    echo "" >> "$OUTPUT_FILE"
}

# Initial process check
echo "Checking initial process state..."
if ! get_process_info; then
    echo "Process $PID not found!"
    exit 1
fi

# Get initial system state
get_system_info

# Send SIGINFO to get process status (macOS specific)
echo "Sending SIGINFO signal to process $PID..."
kill -INFO $PID 2>/dev/null && echo "SIGINFO sent successfully" || echo "Failed to send SIGINFO"

# Monitor for a few iterations
echo "Monitoring process for 30 seconds..."
for i in {1..6}; do
    echo "Sample $i/6..."
    sleep 5
    
    if ! get_process_info; then
        echo "Process terminated during monitoring"
        break
    fi
    
    get_system_info
done

# Final summary
echo "" >> "$OUTPUT_FILE"
echo "Monitoring completed at $(date)" >> "$OUTPUT_FILE"

echo ""
echo "Monitoring completed!"
echo "Results saved to: $OUTPUT_FILE"

# Show summary
echo ""
echo "Process Summary:"
if ps -p $PID > /dev/null 2>&1; then
    echo "Status: Still running"
    ps -p $PID -o pid,user,%cpu,%mem,time,command
else
    echo "Status: Terminated"
fi
