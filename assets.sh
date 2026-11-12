#!/bin/sh
# Redraws the tiles from src/tile.rs, then rasterizes them for the Dock; needs rsvg-convert (librsvg).
set -eu
cd "$(dirname "$0")"
REGEN_ASSETS=1 cargo test -q assets_match_the_drawing
for a in sleepy open blink needs; do
  rsvg-convert -w 512 -h 512 "assets/$a.svg" -o "assets/$a.png"
done
