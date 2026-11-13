#!/bin/sh
# Builds the journal server (the release build) into journal/target/release.
#
# The built program does not see journal/.cargo/config.toml. To serve the web frontend, give
# it --frontend-dir, or copy dist/ of the web frontend to target/release/frontend/.
set -eu
cd "$(dirname "$0")"
exec cargo build --release -p journal-server
