#!/usr/bin/env bash
set -euo pipefail

PROTO_DIR="../../proto/src/main/proto"
OUT_DIR="src/proto"

mkdir -p "$OUT_DIR"

npx pbjs \
  -t static-module \
  -w es6 \
  --no-verify \
  --no-convert \
  --no-delimited \
  -o "$OUT_DIR/bundle.js" \
  "$PROTO_DIR/common.proto" \
  "$PROTO_DIR/page.proto" \
  "$PROTO_DIR/block.proto" \
  "$PROTO_DIR/search.proto" \
  "$PROTO_DIR/api.proto"

npx pbts -o "$OUT_DIR/bundle.d.ts" "$OUT_DIR/bundle.js"

echo "Proto bundle generated in $OUT_DIR"
