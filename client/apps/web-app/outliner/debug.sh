#!/bin/sh
# Builds the web frontend into dist/: a fast build with larger files, for development.
#
# The journal server shows it at http://localhost:8087. Start the server with
# journal/journal-server/debug.sh. It need not restart: reload the browser after the build.
set -eu
cd "$(dirname "$0")"
exec trunk build
