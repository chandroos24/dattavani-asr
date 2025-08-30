#!/bin/bash

# System monitoring script for macOS
# Captures CPU and GPU stats for 20 minutes

OUTPUT_FILE="system_stats_$(date +%Y%m%d_%H%M%S).log"
DURATION=1200  # 20 minutes in seconds
INTERVAL=5     # Sample every 5 seconds

echo "Starting system monitoring for 20 minutes..."
echo "Output file: $OUTPUT_FILE"
echo "Sampling every $INTERVAL seconds"
echo ""

# Initialize log file with headers
cat > "$OUTPUT_FILE" << EOF
System Performance Monitoring - Started at $(date)
Monitoring Duration: 20 minutes
Sample Interval: ${INTERVAL} seconds
========================================

EOF

# Function to get CPU stats
get_cpu_stats() {
    echo "=== CPU Stats - $(date) ===" >> "$OUTPUT_FILE"
    
    # CPU usage via top
    top -l 1 -n 0 | grep "CPU usage" >> "$OUTPUT_FILE"
    
    # Load averages
    echo "Load Averages: $(uptime | awk -F'load averages:' '{print $2}')" >> "$OUTPUT_FILE"
    
    # CPU temperature (if available)
    if command -v powermetrics >/dev/null 2>&1; then
        sudo powermetrics -n 1 -i 1000 --samplers smc -a --hide-cpu-duty-cycle 2>/dev/null | grep -E "(CPU die temperature|Package Power)" >> "$OUTPUT_FILE" 2>/dev/null || echo "CPU temperature: Not available" >> "$OUTPUT_FILE"
    fi
    
    echo "" >> "$OUTPUT_FILE"
}

# Function to get GPU stats
get_gpu_stats() {
    echo "=== GPU Stats - $(date) ===" >> "$OUTPUT_FILE"
    
    # GPU usage via powermetrics (requires sudo)
    if command -v powermetrics >/dev/null 2>&1; then
        sudo powermetrics -n 1 -i 1000 --samplers gpu_power 2>/dev/null | grep -E "(GPU|Graphics)" >> "$OUTPUT_FILE" 2>/dev/null || echo "GPU stats via powermetrics: Not available" >> "$OUTPUT_FILE"
    fi
    
    # Alternative GPU monitoring via system_profiler
    if [[ $(system_profiler SPDisplaysDataType 2>/dev/null | grep -c "Chipset Model") -gt 0 ]]; then
        echo "GPU Info:" >> "$OUTPUT_FILE"
        system_profiler SPDisplaysDataType 2>/dev/null | grep -E "(Chipset Model|VRAM|Metal)" | head -5 >> "$OUTPUT_FILE"
    fi
    
    # Activity Monitor GPU usage (if available)
    if pgrep -x "Activity Monitor" > /dev/null; then
        echo "GPU processes (top 5 by GPU usage):" >> "$OUTPUT_FILE"
        ps aux | head -1 >> "$OUTPUT_FILE"
        ps aux | sort -k3 -nr | head -5 >> "$OUTPUT_FILE"
    fi
    
    echo "" >> "$OUTPUT_FILE"
}

# Function to get memory stats
get_memory_stats() {
    echo "=== Memory Stats - $(date) ===" >> "$OUTPUT_FILE"
    
    # Memory usage
    vm_stat | head -10 >> "$OUTPUT_FILE"
    
    # Memory pressure
    memory_pressure >> "$OUTPUT_FILE" 2>/dev/null || echo "Memory pressure: Not available" >> "$OUTPUT_FILE"
    
    echo "" >> "$OUTPUT_FILE"
}

# Main monitoring loop
start_time=$(date +%s)
end_time=$((start_time + DURATION))
sample_count=0

echo "Monitoring started at $(date)"
echo "Will run until $(date -r $end_time)"

while [ $(date +%s) -lt $end_time ]; do
    sample_count=$((sample_count + 1))
    current_time=$(date +%s)
    elapsed=$((current_time - start_time))
    remaining=$((end_time - current_time))
    
    echo "Sample $sample_count - Elapsed: ${elapsed}s, Remaining: ${remaining}s"
    
    echo "--- Sample $sample_count - $(date) ---" >> "$OUTPUT_FILE"
    
    get_cpu_stats
    get_gpu_stats
    get_memory_stats
    
    echo "========================================" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    
    # Sleep for the interval, but check if we should exit
    if [ $remaining -gt $INTERVAL ]; then
        sleep $INTERVAL
    else
        sleep $remaining
        break
    fi
done

# Final summary
echo "" >> "$OUTPUT_FILE"
echo "Monitoring completed at $(date)" >> "$OUTPUT_FILE"
echo "Total samples collected: $sample_count" >> "$OUTPUT_FILE"
echo "Total duration: $(($(date +%s) - start_time)) seconds" >> "$OUTPUT_FILE"

echo ""
echo "Monitoring completed!"
echo "Results saved to: $OUTPUT_FILE"
echo "Total samples: $sample_count"
