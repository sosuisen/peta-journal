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

Every other argument goes to cargo mutants. --jobs N sets the number of shards; by default
it is one for every 8 GB of memory and at most half the cores. --tmp DIR puts the temporary
folders of the tests in DIR. The shards test the commit at HEAD, so commit the changes first.
The tests in the `timing` modules are skipped; see skip_timing_tests.

The tests wait for the disk, not for the CPU: each makes a database and a search index in a
temporary folder. On macOS the script therefore makes a RAM disk for these folders, and
removes it at the end. With it the tests of the workspace took 14 seconds for 75
(2026-11-18). Elsewhere, --tmp may point at a RAM disk or a fast disk.
"""

import ctypes
import os
import signal
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


def memory_bytes():
    """The memory of this machine, or None when it cannot be read."""
    try:
        if sys.platform == "win32":
            kilobytes = ctypes.c_ulonglong(0)
            ctypes.windll.kernel32.GetPhysicallyInstalledSystemMemory(ctypes.byref(kilobytes))
            return kilobytes.value * 1024 or None
        if sys.platform == "darwin":
            return int(subprocess.run(["sysctl", "-n", "hw.memsize"], check=True,
                                      capture_output=True, text=True).stdout)
        return os.sysconf("SC_PAGE_SIZE") * os.sysconf("SC_PHYS_PAGES")
    except (OSError, ValueError, AttributeError, subprocess.CalledProcessError):
        return None


def default_jobs(memory, cores):
    """One shard for every 8 GB and at most half the cores, and at least one.

    Three shards on a machine with 8 GB and 8 cores were slower than one: a build took 50
    to 150 seconds for 18, and mutants timed out (2026-11-18).
    """
    if memory is None:
        return 1
    return max(1, min(memory // (8 * 1024 ** 3), (cores or 1) // 2))


def parse_args(argv):
    """The number of shards, the folder for temporary files or None, and the arguments for
    cargo mutants with absolute paths."""
    jobs = default_jobs(memory_bytes(), os.cpu_count())
    tmp = None
    rest = []
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "--jobs":
            jobs = int(argv[i + 1])
            i += 2
            continue
        if arg == "--tmp":
            tmp = Path(argv[i + 1]).resolve()
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
    return jobs, tmp, rest


def skip_timing_tests(args):
    """The arguments with the tests of the `timing` modules skipped.

    These tests hold code to a time limit of 100ms or check how long it ran, and fail on a
    machine busy with the other shards (2026-11-18). cargo mutants passes the arguments after
    its own `--` to cargo test, and cargo test passes those after a second `--` to the test
    binaries. The skip goes in that last group. It is not in .cargo/mutants.toml: cargo mutants
    puts those arguments after the ones given here, so a `--` in both would turn the skip into
    a filter that runs only the timing tests.
    """
    if "--" not in args:
        return [*args, "--", "--", "--skip", "timing::"]
    test_args = args[args.index("--") + 1:]
    if "--" in test_args:
        return [*args, "--skip", "timing::"]
    return [*args, "--", "--skip", "timing::"]


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


class RamDisk:
    """A RAM disk on macOS, which lasts as long as the `with` block."""

    MEGABYTES = 512

    def __enter__(self):
        sectors = self.MEGABYTES * 1024 * 1024 // 512
        self.device = subprocess.run(
            ["hdiutil", "attach", "-nomount", f"ram://{sectors}"],
            check=True, capture_output=True, text=True).stdout.strip()
        name = f"mutants-{os.getpid()}"
        subprocess.run(["diskutil", "erasevolume", "APFS", name, self.device],
                       check=True, capture_output=True)
        return Path("/Volumes") / name

    def __exit__(self, *_):
        subprocess.run(["hdiutil", "detach", "-force", self.device], capture_output=True)


def run_shard(crate_dir, k, jobs, args, target, log, tmp, results):
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    if tmp is not None:
        # TMPDIR is read on Unix, TEMP and TMP on Windows
        env.update(TMPDIR=str(tmp), TEMP=str(tmp), TMP=str(tmp))
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
    jobs, tmp, args = parse_args(sys.argv[1:])
    args = skip_timing_tests(args)
    if tmp is None and sys.platform == "darwin":
        # A stop by SIGTERM unwinds as well, so that the RAM disk does not stay
        signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))
        with RamDisk() as disk:
            return run(jobs, disk, args)
    return run(jobs, tmp, args)


def run(jobs, tmp, args):
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
            crate_dir, k, jobs, args, shards / f"target{k}", shards / f"shard{k}.log", tmp,
            results))
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
