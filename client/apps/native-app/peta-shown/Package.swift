// swift-tools-version:5.9
import PackageDescription

// What the two Swift apps share: an item as it is shown, read from the AST that the core gives
// (ADR 041). Each app draws it on its own screen. The types of the border under generated/ are
// made by generate.sh of either app, and both apps take them from here: SwiftPM takes a
// package of one name from one place only.
let package = Package(
    name: "PetaShown",
    platforms: [
        .iOS(.v17),
        .macOS(.v13),
    ],
    products: [
        .library(name: "PetaShown", targets: ["PetaShown"])
    ],
    dependencies: [
        .package(path: "generated/swift/PetaTypes")
    ],
    targets: [
        .target(name: "PetaShown", dependencies: [.product(name: "PetaTypes", package: "PetaTypes")]),
        .testTarget(name: "PetaShownTests", dependencies: ["PetaShown"]),
    ]
)
