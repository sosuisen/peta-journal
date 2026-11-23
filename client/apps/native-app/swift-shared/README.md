# swift-shared

What the two Swift apps share: the view model that the changes of the core make (ADR 076). The
core sends only what changed on the screen; `Applied.swift` keeps the whole view model from
them. The Mac app (`../stickies-mac`) and the iOS app (`../outliner-ios`) both use it.

The Android and Windows apps apply the changes in their own languages (`Applied.kt`,
`Applied.cs`). That the changes make the view model of the core is tested in Rust, in
`client-native-bridge`.

`BrowserImage.swift` makes a pasted HEIC image a PNG, or a JPEG when it has no transparency: a
browser may not show HEIC, and the web shells show the assets too (ADR 071). The iOS app gets
HEIC from Photos and the stickers of the keyboard, the Mac app from a file copied in the Finder.
Each app tests it through its own `PastedImage`.

`generated/swift/PetaTypes` holds the types that cross the border. `generate.sh` of either app
makes it; it is not kept in the repository. Both apps take the types from here, because SwiftPM
takes a package of one name from one place only.

```sh
../stickies-mac/generate.sh   # or ../outliner-ios/generate.sh
swift test
```
