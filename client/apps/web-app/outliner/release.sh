#!/bin/sh
# Builds the web frontend into dist/: small files, for daily use.
#
# The journal server need not restart: reload the browser after the build.
set -eu
cd "$(dirname "$0")"
exec trunk build --release
