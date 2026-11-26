# shell-check

Checks that the shells go through the core and treat its values as it does. The checks are of
two kinds (ADR 090). The static checks, S1 to S4, read the sources of the shells (ADR 064). The
dynamic checks, D1 and D2, are common tests that every shell runs on examples that the core
writes (ADR 089). For them, this crate checks that every shell has the tests.

`cargo test` runs the checks on this repository, on the machine of the developer and in CI
(`tests/repository.rs`). It checks every shell that has rules in `rules/`. When a check finds
something, the test fails and prints what it found.

To run a check by hand:

```
cd peta-journal/client
cargo run -q -p shell-check -- s2 stickies-mac
cargo run -q -p shell-check -- common stickies-mac
```

The exit code is 0 when the check found nothing, 1 when it found something, and 2 when it
could not run.

To run a check on an older commit, check the commit out in another folder and pass it with
`--root <folder>`. The rules are read from this crate unless `--rules <folder>` names others.

## The rules

The rules are TOML files in `rules/`, one for each shell, named after the shell.
`rules/core.toml` holds what is the same for every shell. Each entry that leaves something out
gives the reason in one line.

## S2: every kind of `Event` is named

S2 reads the kinds of `Event` from `client/libs/client-core/src/app.rs`. Each kind must be
named in the sources of the shell, or be in an exclusion list with its reason. It prints each
kind that is neither as `missing: <kind>`. It prints each name in an exclusion list that is no
kind of `Event` as `unknown exclusion: <name>`.

`rules/core.toml` excludes the kinds that the core sends to itself, such as an answer of the
server. The rules of a shell exclude the kinds that this shell need not name.

The check reads text, not types. In Swift, a kind is named as an implicit member, such as
`.pasteText`. A member of another enum with the same name, such as the `.redo` of
`SelectionKey`, counts as the kind too.

In C# and Kotlin, a kind is named as a nested class of `Event`, such as `Event.PasteText(...)`.
`Event` must be a whole name: the `Back` of `KeyEvent.Back` does not count.

In TypeScript, a kind is named as the string under the key `kind`, such as
`{ kind: 'PasteText', ... }`, or as the generated function that makes it, such as
`eventPasteText(...)`. The same string under another key does not count.

In Rust, a kind is named as a variant of `Event`, such as `Event::PasteText { ... }`. `Event`
must be a whole name: the `Open` of `SseEvent::Open` does not count. The Emacs outliner is
checked in its Rust part only: the Lisp sends the lines of `src/protocol.rs`, and the Rust part
makes an `Event` of each.

The `language` of the rules of a shell is `swift`, `csharp`, `kotlin`, `typescript` or `rust`.
It also says which files are read: `.swift`, `.cs`, `.kt`, `.ts` and `.tsx`, or `.rs`. In
TypeScript the tests, `.test.ts` and `.test.tsx`, are not read.

The `skipped` folders of the rules of a shell are not read, though they are under `sources`.
They hold what is not the shell's own code, such as the types generated from the core, which
name every kind.

## The common tests

`rules/core.toml` names the common tests under `[common]`, each with the file of its examples
in `common/`. Every shell says, for each common test, one of these in its rules:

- `test`: the file of its test, from the root of the repository.
- `pending`: the issue that writes the test. The shell passes until then.
- `excluded`: why the shell needs no such test, in one line.

`[common]` of a shell also names, as `tests`, the folders whose tests the test tool of the shell
runs. The check prints a line for each of these:

- A common test of which the shell says none of the three, or more than one.
- A `pending` with 0 as the issue.
- A name that is no common test of `rules/core.toml`.
- A test file that is not there.
- A test file that the test tool does not run: it is outside the folders of `tests`, or in
  TypeScript its name does not end in `.test.ts` or `.test.tsx`.
- A Rust test file that no `mod` in its folder declares, so cargo does not build it.
- A test file that does not name the file of its examples, so it cannot read them.

The test itself checks that it ran every example of the file. `scripts/verification.test.mjs`
checks that the job of every shell runs its tests, in `COMMON_TESTS_STEP`.
