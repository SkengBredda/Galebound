#!/usr/bin/env sh
# Draws every class in every pose into pose_preview.png (see CHARACTERS.md).
cd "$(dirname "$0")" || exit 1
cargo test --release -p galebound -- strikes_land_in_their_hitboxes idle_is_relaxed
cargo test --release -p galebound pose_preview -- --ignored || exit 1
echo "Wrote pose_preview.png"
(xdg-open pose_preview.png >/dev/null 2>&1 &) || true
