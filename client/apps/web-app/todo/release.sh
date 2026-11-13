#!/bin/sh
# Builds the app into dist/, checked by the TypeScript compiler first.
set -eu
cd "$(dirname "$0")"
[ -d node_modules ] || npm install
exec npm run build
