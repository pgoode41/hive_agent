#!/bin/bash
# Quick cleanup script - keep only the last N images

MAX_IMAGES=${1:-20}
IMAGE_DIR="/home/nibbles/Documents/hive_agent/generated_image_captures"

echo "🧹 Cleaning up images (keeping last $MAX_IMAGES)..."

# Get list of images sorted by number
cd "$IMAGE_DIR"
TOTAL=$(ls -1 captured_image_*.png 2>/dev/null | wc -l)

if [ $TOTAL -gt $MAX_IMAGES ]; then
    TO_DELETE=$((TOTAL - MAX_IMAGES))
    echo "📊 Found $TOTAL images, deleting $TO_DELETE oldest ones..."
    
    # Delete oldest images
    ls -1 captured_image_*.png | \
        sort -t_ -k3 -n | \
        head -n $TO_DELETE | \
        xargs rm -f
    
    echo "✅ Cleanup complete!"
else
    echo "✅ Only $TOTAL images found (under limit)"
fi

ls -1 captured_image_*.png 2>/dev/null | wc -l | xargs -I {} echo "📷 Images remaining: {}"
