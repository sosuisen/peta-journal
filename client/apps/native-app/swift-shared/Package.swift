// swift-tools-version:5.9
import PackageDescription

// What the two Swift apps share: the view model that the changes of the core make (ADR 076),
// and a pasted HEIC image as a browser shows it (ADR 071).
// The types of the border under generated/ are made by generate.sh of either app, and both apps
// take them from here: SwiftPM takes a package of one name from one place only.
let package = Package(
    name: "PetaShared",
    platforms: [
        .iOS(.v17),
        .macOS(.v13),
    ],
    products: [
        .library(name: "PetaShared", targets: ["PetaShared"])
    ],
    dependencies: [
        .package(path: "generated/swift/PetaTypes")
    ],
    targets: [
        .target(name: "PetaShared", dependencies: [.product(name: "PetaTypes", package: "PetaTypes")]),
        .testTarget(
            name: "PetaSharedTests",
            dependencies: ["PetaShared", .product(name: "PetaTypes", package: "PetaTypes")]
        ),
    ]
)
