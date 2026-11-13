#!/bin/sh
# Builds the web frontend into frontend-rust/crates/client-shell-leptos/dist. It works from any folder.
#
#   ./build-frontend.sh          small files, for daily use
#   ./build-frontend.sh --dev    a faster build with larger files, for development
#
# The journal server need not restart: reload the browser after the build.
set -eu
here=$(cd "$(dirname "$0")" && pwd)

mode=--release
case "${1:-}" in
    "") ;;
    --dev) mode= ;;
    *) echo "usage: $0 [--dev]" >&2; exit 2 ;;
esac

cd "$here/frontend-rust/crates/client-shell-leptos"
# No quotes: an empty mode must not reach trunk as an argument.
exec trunk build $mode
