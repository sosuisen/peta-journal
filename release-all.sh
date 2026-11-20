#!/bin/sh
# Brings PetaJournal on this machine up to date: pulls, makes the release builds, and starts
# the journal server, then the web frontend in the browser and, on a Mac, PetaStickies.
#
#   ./release-all.sh             pulls the branch first
#   ./release-all.sh --no-pull   builds what is checked out now
#
# The order:
#   1. git pull --ff-only
#   2. PetaStickies ends on a Mac. Then the journal server ends.
#   3. The release builds: the web frontend, the journal server, the Emacs shell, todo and,
#      on a Mac, PetaStickies.
#   4. The journal server starts in the background. Its log is journal/target/release/journal-server.log.
#   5. When the server answers on port 8087, the browser and PetaStickies start.
#
# On Windows (Git Bash) it runs release-all.ps1, which builds the WinUI apps too.
set -eu
root=$(cd "$(dirname "$0")" && pwd)

case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*)
        script=$(cygpath -w "$root/release-all.ps1")
        case "${1:-}" in
            --no-pull) exec pwsh -NoProfile -File "$script" -NoPull ;;
            *) exec pwsh -NoProfile -File "$script" ;;
        esac
        ;;
esac

pull=yes
case "${1:-}" in
    --no-pull) pull=no ;;
    -h | --help)
        sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'
        exit 0
        ;;
esac

port=8087
apps=$root/client/apps
release=$root/journal/target/release
mac=no
[ "$(uname -s)" = Darwin ] && mac=yes

step() {
    printf '==> %s\n' "$1"
}

answers() {
    curl -s -o /dev/null "http://localhost:$port/"
}

if [ "$pull" = yes ]; then
    step Pull
    git -C "$root" pull --ff-only
fi

# The apps end before the server: they save what is typed to the server when they close.
# Why not pkill: quit lets PetaStickies save the text that waits, and keep its stickies
if [ "$mac" = yes ] && pgrep -x PetaStickies >/dev/null; then
    echo 'Closing PetaStickies'
    osascript -e 'quit app "PetaStickies"'
    while pgrep -x PetaStickies >/dev/null; do sleep 0.2; done
fi
# SIGINT, as Ctrl+C: the server writes the search index before it ends
if pgrep -x journal-server >/dev/null; then
    echo 'Ending the journal server'
    pkill -INT -x journal-server
    while pgrep -x journal-server >/dev/null; do sleep 0.2; done
fi
if answers; then
    echo "Another program uses port $port. End it first." >&2
    exit 1
fi

step 'Web frontend'
"$apps/web-app/outliner/release.sh"
step 'Journal server'
"$root/journal/journal-server/release.sh"
# Why a copy and not --frontend-dir: a debug `trunk build` in dist/ would change the frontend
# of the running release server too
step 'Copy the web frontend'
rm -rf "$release/frontend"
cp -R "$apps/web-app/outliner/dist" "$release/frontend"
# The Emacs shell is not ended: Emacs would lose its buffers
if pgrep -x outliner-emacs >/dev/null; then
    echo 'The Emacs shell runs: its build is left out. Close the PetaJournal buffers in Emacs first.' >&2
else
    step 'Emacs shell'
    "$apps/emacs/outliner/release.sh"
fi
step todo
"$apps/web-app/todo/release.sh"
if [ "$mac" = yes ]; then
    # --generate: the pull may have changed the Rust core under the types of Swift
    step PetaStickies
    "$apps/native-app/stickies-mac/release.sh" --generate
fi

step 'Starting the journal server'
cd "$release"
nohup ./journal-server >journal-server.log 2>&1 &
server=$!
tries=0
until answers; do
    if ! kill -0 "$server" 2>/dev/null; then
        echo "The journal server ended. See $release/journal-server.log" >&2
        exit 1
    fi
    tries=$((tries + 1))
    if [ "$tries" -gt 200 ]; then
        echo "The journal server does not answer on port $port" >&2
        exit 1
    fi
    sleep 0.3
done

step 'Starting the apps'
if [ "$mac" = yes ]; then
    open "http://localhost:$port"
    open "$apps/native-app/stickies-mac/build/Build/Products/Release/PetaStickies.app"
elif command -v xdg-open >/dev/null; then
    xdg-open "http://localhost:$port" >/dev/null 2>&1 || true
fi
echo "Done. The journal server runs at http://localhost:$port"
