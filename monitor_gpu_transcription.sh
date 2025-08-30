#!/bin/bash

# GPU monitoring script for transcription testing
OUTPUT_FILE="gpu_transcription_test_$(date +%Y%m%d_%H%M%S).log"
AUDIO_FILE="${1:-test_audio.wav}"

echo "GPU Transcription Monitoring Test"
echo "Audio file: $AUDIO_FILE"
echo "Output file: $OUTPUT_FILE"
echo "================================"

# Initialize log file
cat > "$OUTPUT_FILE" << EOF
GPU Transcription Monitoring - Started at $(date)
Audio File: $AUDIO_FILE
========================================

EOF

# Function to get GPU stats
get_gpu_stats() {
    echo "=== GPU Stats - $(date) ===" >> "$OUTPUT_FILE"
    
    # GPU usage via powermetrics
    if command -v powermetrics >/dev/null 2>&1; then
        sudo powermetrics -n 1 -i 500 --samplers gpu_power 2>/dev/null | grep -E "(GPU|Graphics)" >> "$OUTPUT_FILE" 2>/dev/null || echo "GPU stats: Not available" >> "$OUTPUT_FILE"
    fi
    
    echo "" >> "$OUTPUT_FILE"
}

# Start background monitoring
echo "Starting background GPU monitoring..."
while true; do
    get_gpu_stats
    sleep 2
done &
MONITOR_PID=$!

# Activate whisper environment and run transcription
echo "Starting transcription with GPU acceleration..."
source whisper_env/bin/activate

echo "--- Transcription Start - $(date) ---" >> "$OUTPUT_FILE"

# Run transcription with GPU
time whisper "$AUDIO_FILE" --model large-v3 --device mps --compute_type float16 --output_format json --output_dir /tmp --language en 2>&1 | tee -a "$OUTPUT_FILE"

echo "--- Transcription End - $(date) ---" >> "$OUTPUT_FILE"

# Stop monitoring
kill $MONITOR_PID 2>/dev/null

echo ""
echo "Transcription completed!"
echo "GPU monitoring log saved to: $OUTPUT_FILE"

# Show final GPU stats
echo ""
echo "Final GPU usage summary:"
grep -E "(GPU HW active|GPU Power|GPU idle)" "$OUTPUT_FILE" | tail -10
