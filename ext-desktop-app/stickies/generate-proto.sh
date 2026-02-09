#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROTO_DIR="$SCRIPT_DIR/../../proto/src/main/proto"
OUT_DIR="$SCRIPT_DIR/Proto"

mkdir -p "$OUT_DIR"

# Try protoc in PATH first, then fall back to Maven-cached protoc
if command -v protoc &>/dev/null; then
    PROTOC=protoc
else
    PROTOC=$(find "$SCRIPT_DIR/../../proto/target" -name "protoc*.exe" 2>/dev/null | head -1)
    if [ -z "$PROTOC" ]; then
        echo "ERROR: protoc not found. Either install it or run 'cd proto && mvn compile' first."
        exit 1
    fi
fi

echo "Using protoc: $PROTOC"
echo "Generating C# protobuf classes..."

"$PROTOC" \
    --proto_path="$PROTO_DIR" \
    --csharp_out="$OUT_DIR" \
    "$PROTO_DIR/common.proto" \
    "$PROTO_DIR/page.proto" \
    "$PROTO_DIR/block.proto" \
    "$PROTO_DIR/search.proto" \
    "$PROTO_DIR/api.proto"

echo "Proto generation complete. Files:"
ls "$OUT_DIR"/*.cs
