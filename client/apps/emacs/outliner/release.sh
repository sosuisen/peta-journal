#!/bin/sh
# Builds the program (the release build) into client/target/release. The mode of Emacs finds
# it there.
set -eu
cd "$(dirname "$0")"
exec cargo build --release -p client-shell-emacs
