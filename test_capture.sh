#!/bin/bash

# Test script for the new capture-and-process functionality
# This demonstrates how to use the new video capture feature

echo "🎬 Testing Dattavani ASR Video Capture and Processing"
echo "======================================================"

# Example usage (replace with your actual video file)
VIDEO_FILE="path/to/your/video.mp4"
START_TIME="00:01:30"  # Start at 1 minute 30 seconds
END_TIME="00:02:00"    # End at 2 minutes
TITLE="sample-meeting"
LANGUAGE="en"

echo "📝 Command that would be executed:"
echo "./target/release/dattavani-asr capture-and-process \\"
echo "  --start-time $START_TIME \\"
echo "  --end-time $END_TIME \\"
echo "  --title $TITLE \\"
echo "  --language $LANGUAGE \\"
echo "  \"$VIDEO_FILE\""

echo ""
echo "📁 This will create a folder structure like:"
echo "/Volumes/ssd1/video-capture/"
echo "└── sample-meeting-20250829_214700/"
echo "    ├── sample-meeting-20250829_214700.mp4  (captured video segment)"
echo "    ├── sample-meeting-20250829_214700.mp3  (extracted audio)"
echo "    └── sample-meeting-20250829_214700.txt  (ASR transcript)"

echo ""
echo "🚀 Features:"
echo "• Captures video segment between specified start and end times"
echo "• Extracts audio as MP3 from the captured video"
echo "• Performs ASR transcription on the audio"
echo "• Saves all files in a timestamped subfolder"
echo "• Uses HH:MM:SS time format for precise timing"
echo "• Supports local files, Google Drive URLs, and GCS URIs"

echo ""
echo "💡 Usage examples:"
echo ""
echo "# Local video file"
echo "./target/release/dattavani-asr capture-and-process \\"
echo "  --start-time 00:05:00 --end-time 00:10:00 \\"
echo "  --title 'presentation-intro' \\"
echo "  --language en \\"
echo "  '/path/to/presentation.mp4'"

echo ""
echo "# Google Drive video"
echo "./target/release/dattavani-asr capture-and-process \\"
echo "  --start-time 00:15:30 --end-time 00:20:45 \\"
echo "  --title 'meeting-discussion' \\"
echo "  --language en \\"
echo "  'https://drive.google.com/file/d/YOUR_FILE_ID/view'"

echo ""
echo "# Custom output folder"
echo "./target/release/dattavani-asr capture-and-process \\"
echo "  --start-time 00:00:30 --end-time 00:03:00 \\"
echo "  --title 'demo-clip' \\"
echo "  --language hi \\"
echo "  --output-folder '/custom/path/captures' \\"
echo "  'video.mp4'"

echo ""
echo "✅ Ready to use! Replace the example paths with your actual video files."
