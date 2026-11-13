# peta-shown

What the two Swift apps share: an item as it is shown, read from the AST that the core gives
with each row (`Row.ast`, ADR 041). The iOS app (`../outliner-ios`) and the Mac app
(`../stickies-mac`) each draw it on a screen of their own.

- `ShownBlocks.swift` makes the parts of an item: text, code blocks, tables, images of the
  assets folder, and the lists and quotes that hold one of these. `LinkTarget` says where a link
  of an item leads: a page in the app, the web, or a file of the assets folder.
- `ShownParagraphs.swift` makes text into paragraphs of pieces with a look each.

A `switch` takes every kind of node, so a kind that the AST gets later stops the build until it
is drawn. The links to pages and tags are written by the core of each app (`PageLinks`): the two
apps have cores of their own.

`generated/swift/PetaTypes` holds the types that cross the border. `generate.sh` of either app
makes it; it is not kept in the repository. Both apps take the types from here, because SwiftPM
takes a package of one name from one place only.

```sh
../stickies-mac/generate.sh   # or ../outliner-ios/generate.sh
swift test
```
