"""Runs cargo-mutants in several shards at once, each in a git worktree of its own.

cargo-mutants runs one mutant at a time with --in-place, which this workspace needs: the
copy it makes otherwise cannot reach peta-atlas, a path dependency outside the workspace.
One mutant waits for its build and its tests and leaves most of the cores idle.

The shards live under target/mutants-shards/ of the main checkout, also when this script
runs from another worktree. They stay there, with a build directory each, so the next run
builds only what changed. `cargo clean` removes them; the next run makes them again.

Usage, from journal:

    git diff --relative main...HEAD > /tmp/pr.diff
    python -I scripts/mutants-parallel.py --in-diff /tmp/pr.diff

Every argument goes to cargo mutants. --jobs N sets the number of shards (3 by default).
The shards test the commit at HEAD, so commit the changes first.
"""

import os
import subprocess
import sys
import threading
import time
from pathlib import Path

CRATE = Path(__file__).resolve().parent.parent
OUTCOMES = ["caught", "missed", "timeout", "unviable"]


def git(*args, cwd=CRATE):
    return subprocess.run(
        ["git", *args], cwd=cwd, check=True, capture_output=True, text=True
    ).stdout.strip()


def parse_args(argv):
    """The number of shards, and the arguments for cargo mutants with absolute paths."""
    jobs = 3
    rest = []
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "--jobs":
            jobs = int(argv[i + 1])
            i += 2
            continue
        rest.append(arg)
        # Each shard runs in its own directory, so a relative path would point elsewhere.
        # The paths of --file stay as they are: cargo mutants matches them in its tree.
        if arg == "--in-diff" and i + 1 < len(argv):
            rest.append(str(Path(argv[i + 1]).resolve()))
            i += 2
            continue
        i += 1
    return jobs, rest


def main_checkout():
    common = Path(git("rev-parse", "--path-format=absolute", "--git-common-dir"))
    return common.parent


def prepare_shard(shards, k, sha, crate_in_repo):
    tree = shards / f"wt{k}"
    if not tree.is_dir():
        git("worktree", "add", "--detach", str(tree), sha)
    else:
        # A shard stopped halfway leaves a mutant in its tree, so the checkout is forced
        git("checkout", "--quiet", "--force", "--detach", sha, cwd=tree)
    return tree / crate_in_repo


def run_shard(crate_dir, k, jobs, args, target, log, results):
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    cmd = ["cargo", "mutants", "--in-place", "--shard", f"{k}/{jobs}",
           "--sharding", "round-robin", *args]
    with open(log, "w", encoding="utf8") as out:
        code = subprocess.run(cmd, cwd=crate_dir, env=env, stdout=out,
                              stderr=subprocess.STDOUT).returncode
    found = {}
    for outcome in OUTCOMES:
        path = crate_dir / "mutants.out" / f"{outcome}.txt"
        found[outcome] = path.read_text(encoding="utf8").splitlines() if path.exists() else []
    results[k] = (code, found)


def main():
    jobs, args = parse_args(sys.argv[1:])
    if git("status", "--porcelain", "--", "."):
        print("The crate has changes that are not committed. Commit them first:"
              " the shards test the commit at HEAD.", file=sys.stderr)
        return 1

    root = main_checkout()
    crate_in_repo = CRATE.relative_to(Path(git("rev-parse", "--show-toplevel")))
    shards = root / crate_in_repo / "target" / "mutants-shards"
    shards.mkdir(parents=True, exist_ok=True)
    git("worktree", "prune")
    sha = git("rev-parse", "HEAD")

    started = time.time()
    results = {}
    threads = []
    for k in range(jobs):
        crate_dir = prepare_shard(shards, k, sha, crate_in_repo)
        t = threading.Thread(target=run_shard, args=(
            crate_dir, k, jobs, args, shards / f"target{k}", shards / f"shard{k}.log", results))
        t.start()
        threads.append(t)
    for t in threads:
        t.join()

    totals = {o: [] for o in OUTCOMES}
    failed = []
    for k in sorted(results):
        code, found = results[k]
        for outcome in OUTCOMES:
            totals[outcome] += found[outcome]
        # cargo mutants exits with 2 for missed mutants and 3 for timeouts; others stop a shard
        if code not in (0, 2, 3):
            failed.append(k)

    minutes = (time.time() - started) / 60
    print(f"{sum(len(v) for v in totals.values())} mutants in {minutes:.1f} min with {jobs} shards: "
          + ", ".join(f"{len(totals[o])} {o}" for o in OUTCOMES))
    for outcome in ("missed", "timeout"):
        for line in totals[outcome]:
            print(f"{outcome.upper():8} {line}")
    for k in failed:
        print(f"Shard {k} stopped; see {shards / f'shard{k}.log'}", file=sys.stderr)
    if failed:
        return 1
    return 2 if totals["missed"] else 0


if __name__ == "__main__":
    sys.exit(main())
