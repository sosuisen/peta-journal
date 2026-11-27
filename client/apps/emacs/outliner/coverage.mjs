// Measures the lines of the Emacs Lisp (lisp/*.el) that the tests run, and fails under the floor.
// The tests are those of the buffer and the E2E tests, in one Emacs: the E2E tests run what the
// tests of the buffer cannot, the commands on real keys and the start of the shell. So this needs
// what the E2E tests need: PETA_EMACS, PETA_E2E_SERVER and a built outliner-emacs.
//
// Usage, in this folder:
//   node coverage.mjs              prints the lines covered of each file, and fails under the floor
//   node coverage.mjs --uncovered  also prints the lines that no test ran
//
// The tool is undercover.el. It and the two libraries it needs are one file each. They are fetched
// once, at the commits below, into client/target/elisp-coverage, and a file with another hash is
// not used. No file of lisp/ is left out: lisp/test/peta-journal-coverage.el names what is measured.

import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const FLOOR = 90;

// A new version changes nothing here until these lines ask for it
const TOOLS = [
  // v0.8.1
  {
    file: 'undercover.el',
    repository: 'undercover-el/undercover.el',
    commit: '1d3587f1fad66a747688f36636b67b33b73447d3',
    sha256: 'b3e4111b0b5ae46f5e8c4290b44f9f60826c109b2a9d9c9be920fc704e49af9b',
  },
  // 2.20.0
  {
    file: 'dash.el',
    repository: 'magnars/dash.el',
    commit: 'fcb5d831fc08a43f984242c7509870f30983c27c',
    sha256: '96af5bb2917c05aa42b67eb0b39bf8789ecb4df66a7e67751a3ef5a7c51e8f94',
  },
  // v0.3.3
  {
    file: 'shut-up.el',
    repository: 'cask/shut-up',
    commit: '081d6b01e3ba0e60326558e545c4019219e046ce',
    sha256: 'b0f38432d4f2f17c390f59dff723f54df843839b01c496d3d7fa156f733905e4',
  },
];

const HERE = resolve(fileURLToPath(import.meta.url), '..');
const TOOLS_DIR = resolve(HERE, '..', '..', '..', 'target', 'elisp-coverage');

function fail(message) {
  console.error(message);
  process.exit(1);
}

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

// Puts each tool in TOOLS_DIR, unless the file with the right hash is there already
async function fetchTools() {
  mkdirSync(TOOLS_DIR, { recursive: true });
  for (const { file, repository, commit, sha256: expected } of TOOLS) {
    const path = join(TOOLS_DIR, file);
    if (existsSync(path) && sha256(readFileSync(path)) === expected) continue;
    const url = `https://raw.githubusercontent.com/${repository}/${commit}/${file}`;
    const response = await fetch(url);
    if (!response.ok) fail(`${url}: ${response.status}`);
    const bytes = Buffer.from(await response.arrayBuffer());
    if (sha256(bytes) !== expected) fail(`${url}: the hash is ${sha256(bytes)}, not ${expected}`);
    writeFileSync(path, bytes);
  }
}

// The lines of each file in a report of the LCOV format: { name: [[line, count], ...] }
function linesOf(report) {
  const files = {};
  let lines;
  for (const row of report.split(/\r?\n/)) {
    if (row.startsWith('SF:')) lines = files[basename(row.slice(3).replaceAll('\\', '/'))] = [];
    const counted = row.match(/^DA:(\d+),(\d+)/);
    if (counted) lines.push([Number(counted[1]), Number(counted[2])]);
  }
  return files;
}

const emacs = process.env.PETA_EMACS;
if (!emacs) fail('PETA_EMACS is not set');
// Without it the E2E tests skip themselves, and their lines would count as not covered
if (!process.env.PETA_E2E_SERVER) fail('PETA_E2E_SERVER is not set');

await fetchTools();

const work = mkdtempSync(join(tmpdir(), 'peta-elisp-coverage-'));
const reportFile = join(work, 'lcov.info');
// UNDERCOVER_CONFIG could leave files out
const { UNDERCOVER_CONFIG: _, ...env } = process.env;
const tests = spawnSync(
  emacs,
  [
    ...['-Q', '--batch', '-L', TOOLS_DIR, '-L', 'lisp', '-L', 'lisp/test'],
    ...['-l', 'lisp/test/peta-journal-coverage.el'],
    ...['-l', 'lisp/test/peta-journal-test.el', '-l', 'lisp/test/peta-journal-e2e.el'],
    ...['-f', 'ert-run-tests-batch-and-exit'],
  ],
  { cwd: HERE, stdio: 'inherit', env: { ...env, PETA_COVERAGE_REPORT: reportFile } },
);
if (tests.status !== 0) fail(`the tests failed${tests.error ? `: ${tests.error.message}` : ''}`);

const files = linesOf(readFileSync(reportFile, 'utf8'));
rmSync(work, { recursive: true });

const sources = readdirSync(join(HERE, 'lisp')).filter(name => name.endsWith('.el'));
const unmeasured = sources.filter(name => !files[name]?.length);
if (unmeasured.length > 0) fail(`not measured: ${unmeasured.join(', ')}`);

let hit = 0;
let total = 0;
const percent = (part, whole) => `${((100 * part) / whole).toFixed(2)}%`;
for (const name of sources) {
  const covered = files[name].filter(([, count]) => count > 0).length;
  console.log(`${name}: ${covered}/${files[name].length} lines (${percent(covered, files[name].length)})`);
  if (process.argv.includes('--uncovered')) {
    console.log(
      `  not covered: ${files[name]
        .filter(([, count]) => count === 0)
        .map(([line]) => line)
        .join(' ')}`,
    );
  }
  hit += covered;
  total += files[name].length;
}
console.log(`total: ${hit}/${total} lines (${percent(hit, total)})`);
if (hit * 100 < FLOOR * total) fail(`the coverage of the lines is under ${FLOOR}%`);
