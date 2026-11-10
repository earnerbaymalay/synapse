#!/usr/bin/env bash
# Usage: scripts/make_readme_hero.sh <input.mp4>
# Builds the README hero assets from the PLEXUS neon logo reveal:
#   docs/assets/plexus-neon-hero.gif  (640px wide, 15fps, 2-pass palette, loops forever, <6 MB)
#   docs/assets/plexus-neon-hero.mp4  (copy of the source, linked from the GIF)
#   docs/assets/plexus-neon-hero.png  (final lit frame: shown instead of the GIF to readers
#                                      who have reduced motion turned on)
set -euo pipefail

IN="${1:-}"
[ -n "$IN" ] && [ -f "$IN" ] || { echo "usage: $0 <input.mp4>" >&2; exit 1; }

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT/docs/assets"
GIF="$OUT_DIR/plexus-neon-hero.gif"
MP4="$OUT_DIR/plexus-neon-hero.mp4"
STILL="$OUT_DIR/plexus-neon-hero.png"
MAX_BYTES=$((6 * 1024 * 1024))
mkdir -p "$OUT_DIR"

PAL="$(mktemp --suffix=.png)"
trap 'rm -f "$PAL"' EXIT

encode() { # fps colors
  local fps="$1" colors="$2" filt="fps=$1,scale=640:-1:flags=lanczos"
  ffmpeg -v error -y -i "$IN" -vf "$filt,palettegen=max_colors=$colors:stats_mode=diff" "$PAL"
  ffmpeg -v error -y -i "$IN" -i "$PAL" \
    -lavfi "$filt [x]; [x][1:v] paletteuse=dither=sierra2_4a:diff_mode=rectangle" \
    -loop 0 "$GIF"
}

# Try progressively lighter settings until under the size cap.
for combo in "15 256" "15 192" "15 128" "12 128" "10 96"; do
  set -- $combo
  encode "$1" "$2"
  size=$(stat -c %s "$GIF")
  echo "fps=$1 colors=$2 -> $size bytes"
  [ "$size" -le "$MAX_BYTES" ] && break
done

[ "$(stat -c %s "$GIF")" -le "$MAX_BYTES" ] || echo "WARNING: GIF still over 6 MB" >&2
cp -f "$IN" "$MP4"
# still for prefers-reduced-motion: last frame, where the sign is fully lit
ffmpeg -v error -y -sseof -0.1 -i "$IN" -frames:v 1 -update 1 -vf "scale=640:-1:flags=lanczos" "$STILL"
echo "wrote $GIF, $MP4 and $STILL"
