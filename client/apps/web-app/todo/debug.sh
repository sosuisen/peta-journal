#!/bin/sh
# Starts the development server of Vite, at http://localhost:5180. What is given goes to Vite.
#
# The journal server has to run already. Open http://localhost:5180/?port=8087 to call it.
set -eu
cd "$(dirname "$0")"
# Every time: a pull may add packages, which an old node_modules lacks
npm install
exec npm run dev -- "$@"
