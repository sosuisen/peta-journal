#!/bin/sh
# Builds the journal server (the debug build) and starts it on port 8087. What is given goes
# to the server.
#
#   ./debug.sh
#   ./debug.sh --help
#
# It serves the web frontend from client/apps/web-app/outliner-react/dist (journal/.cargo/config.toml).
set -eu
cd "$(dirname "$0")"
exec cargo run -p journal-server -- "$@"
