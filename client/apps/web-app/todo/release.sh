#!/bin/sh
# Builds the app into dist/, checked by the TypeScript compiler first.
set -eu
cd "$(dirname "$0")"
# Every time: a pull may add packages, which an old node_modules lacks
npm install
exec npm run build
