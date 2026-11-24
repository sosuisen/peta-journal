# shell-check

Checks by reading the sources that the shells go through the core. ADR 064 says what the
checks are and how they are tried. They are run by hand, not in CI, while they are tried.

```
cd peta-journal/client
cargo run -q -p shell-check -- s2 stickies-mac
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
