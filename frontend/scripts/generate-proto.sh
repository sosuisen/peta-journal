#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FRONTEND_DIR="$(dirname "$SCRIPT_DIR")"
PROTO_DIR="$FRONTEND_DIR/../proto/src/main/proto"
OUT_DIR="$FRONTEND_DIR/src/app/core/proto"

mkdir -p "$OUT_DIR"

# Generate static JS module
npx pbjs \
  -t static-module \
  -w es6 \
  --es6 \
  -o "$OUT_DIR/bundle.js" \
  "$PROTO_DIR/common.proto" \
  "$PROTO_DIR/page.proto" \
  "$PROTO_DIR/block.proto" \
  "$PROTO_DIR/search.proto" \
  "$PROTO_DIR/api.proto"

# Generate TypeScript definitions
npx pbts \
  -o "$OUT_DIR/bundle.d.ts" \
  "$OUT_DIR/bundle.js"

echo "Proto TypeScript bindings generated in $OUT_DIR"
