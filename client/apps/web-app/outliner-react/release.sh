#!/bin/sh
# Builds the web frontend into dist/: the wasm of the core and the types of the border first,
# then the app, checked by the TypeScript compiler.
#
# The journal server need not restart: reload the browser after the build.
set -eu
cd "$(dirname "$0")"
[ -d node_modules ] || npm install
exec npm run build
