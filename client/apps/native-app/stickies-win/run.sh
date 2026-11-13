#!/bin/sh
# Builds the app and starts it. A running app is ended first: it holds its files, and the build
# could not write them. It is run.ps1 for Git Bash.
#
#   ./run.sh                          the stickies of the run before, of the server at http://localhost:8087
#   ./run.sh -url /pages/notes        a page; -url /today is today's journal
#   ./run.sh -server http://localhost:8087 -url /pages/notes
#   ./run.sh --generate ...           makes generated/ again first, after a change in Rust
#   ./run.sh --debug ...              the Debug build
#
# The journal server has to run already.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"

configuration=Release
generate=no
while [ $# -gt 0 ]; do
    case "$1" in
        --generate) generate=yes ;;
        --debug) configuration=Debug ;;
        -h | --help)
            sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) break ;;
    esac
    shift
done

if [ "$generate" = yes ] || [ ! -d generated ]; then
    ./generate.sh
fi

# Two slashes: Git Bash would turn an option with one slash into a path
taskkill //IM PetaStickies.exe //F >/dev/null 2>&1 || true

dotnet build PetaStickies -c "$configuration" -nologo -v q

"PetaStickies/bin/$configuration/net10.0-windows10.0.22621.0/win-x64/PetaStickies.exe" "$@" &
