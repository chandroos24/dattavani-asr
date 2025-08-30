#!/bin/bash

# Show help
show_help() {
    cat << EOF
Usage: $0 <start_time> <end_time> <video_url>

Capture and process YouTube video segments with Tagalog ASR transcription.

Arguments:
    start_time    Start time in HH:MM:SS format (e.g., 00:01:30)
    end_time      End time in HH:MM:SS format (e.g., 00:05:00)
    video_url     YouTube video URL or local video file path

Examples:
    $0 00:01:30 00:05:00 "https://www.youtube.com/watch?v=VIDEO_ID"
    $0 00:10:00 00:15:30 "/path/to/video.mp4"

Options:
    -h, --help    Show this help message

EOF
}

# Check for help flag
if [[ "$1" == "-h" || "$1" == "--help" ]]; then
    show_help
    exit 0
fi

# Validate arguments
if [[ $# -ne 3 ]]; then
    echo "Error: Expected 3 arguments, got $#" >&2
    echo "Use -h or --help for usage information" >&2
    exit 1
fi

# Validate time format (basic check)
if ! [[ "$1" =~ ^[0-9]{2}:[0-9]{2}:[0-9]{2}$ ]]; then
    echo "Error: Invalid start time format. Use HH:MM:SS (e.g., 00:01:30)" >&2
    exit 1
fi

if ! [[ "$2" =~ ^[0-9]{2}:[0-9]{2}:[0-9]{2}$ ]]; then
    echo "Error: Invalid end time format. Use HH:MM:SS (e.g., 00:05:00)" >&2
    exit 1
fi

# Check if video URL/path is provided
if [[ -z "$3" ]]; then
    echo "Error: Video URL or path cannot be empty" >&2
    exit 1
fi

# Change to project directory and run
cd /Volumes/ssd1/projects/dattavani/dattavani-asr-rust || {
    echo "Error: Failed to change to project directory" >&2
    exit 1
}

# Check if binary exists
if [[ ! -f "./target/release/dattavani-asr" ]]; then
    echo "Error: dattavani-asr binary not found. Run 'cargo build --release' first." >&2
    exit 1
fi

# Execute with error handling
PATH="/Volumes/ssd1/projects/dattavani/dattavani-asr-rust/whisper_simple/bin:$PATH" \
./target/release/dattavani-asr capture-and-process \
  --start-time "$1" \
  --end-time "$2" \
  --title "youtube-live-segment" \
  --language tl \
  "$3"

# Check exit status
if [[ $? -ne 0 ]]; then
    echo "Error: ASR processing failed" >&2
    exit 1
fi
