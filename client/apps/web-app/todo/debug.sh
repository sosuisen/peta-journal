#!/bin/sh
# Starts the development server of Vite, at http://localhost:5180. What is given goes to Vite.
#
# The journal server has to run already.
set -eu
cd "$(dirname "$0")"
[ -d node_modules ] || npm install
exec npm run dev -- "$@"
