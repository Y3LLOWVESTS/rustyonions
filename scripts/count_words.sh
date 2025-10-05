#!/usr/bin/env bash
# Count words in docs/ for selected crates, excluding docs/all_docs/ and docs/dump/ folders.
# Includes ALL_DOCS.md (we only exclude the folder). Case-insensitive *.md/*.MD.
# macOS/BSD compatible. Run from repo root.

set -euo pipefail

crates=(
  micronode
  macronode
  ron-auth
  ron-bus
  ron-kernel
  ron-proto
  ron-transport
  ryker
  svc-gateway
  svc-index
  svc-overlay
)

printf "\nScanning docs for selected crates (excludes docs/all_docs/ and docs/dump/ folders; includes ALL_DOCS.md files)\n\n"
printf "%-15s %12s %10s %10s\n" "Crate" "Words" "Pages@500" "Pages@350"
printf "%-15s %12s %10s %10s\n" "---------------" "------------" "----------" "----------"

grand_total=0

for c in "${crates[@]}"; do
  dir="crates/$c/docs"
  words=0
  if [ -d "$dir" ]; then
    # Find all *.md (case-insensitive), exclude the all_docs/ and dump/ subfolders only.
    # We sum with wc -w; tail -n1 grabs the "total" line if multiple files, or the single file line otherwise.
    words=$(find "$dir" \
      -type d \( -name all_docs -o -name dump \) -prune -o \
      -type f \( -iname '*.md' \) -print0 \
      | xargs -0 wc -w 2>/dev/null \
      | tail -n1 | awk '{print ($1 ~ /^[0-9]+$/) ? $1 : 0}')
  fi

  pages500=$(( (words + 499) / 500 ))
  pages350=$(( (words + 349) / 350 ))
  printf "%-15s %12d %10d %10d\n" "$c" "$words" "$pages500" "$pages350"

  grand_total=$((grand_total + words))
done

printf "%-15s %12s %10s %10s\n" "---------------" "------------" "----------" "----------"
total500=$(( (grand_total + 499) / 500 ))
total350=$(( (grand_total + 349) / 350 ))
printf "%-15s %12d %10d %10d\n\n" "TOTAL" "$grand_total" "$total500" "$total350"

echo "Per-file breakdown for a crate (example: svc-index):"
echo "find crates/svc-index/docs -type d \\( -name all_docs -o -name dump \\) -prune -o -type f -iname '*.md' -print0 | xargs -0 wc -w | sort -n"
