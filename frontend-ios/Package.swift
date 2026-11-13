// swift-tools-version:5.9
import PackageDescription

// The packages under generated/ are made from the Rust crates by generate.sh.
let package = Package(
    name: "PetaJournal",
    platforms: [
        .iOS(.v17),
        // The parts without a screen are tested on the Mac, with no simulator
        .macOS(.v14),
    ],
    products: [
        .library(name: "PetaSync", targets: ["PetaSync"]),
        .library(name: "PetaBorder", targets: ["PetaBorder"]),
        .library(name: "PetaDropbox", targets: ["PetaDropbox"]),
    ],
    dependencies: [
        .package(path: "generated/PetaCore"),
        .package(path: "generated/swift/PetaTypes"),
        .package(url: "https://github.com/dropbox/SwiftyDropbox.git", exact: "10.2.4"),
    ],
    targets: [
        // The sync with the cloud. It knows neither the core nor Dropbox
        .target(name: "PetaSync"),
        .testTarget(name: "PetaSyncTests", dependencies: ["PetaSync"]),
        // The Markdown folder in Dropbox, with the SDK of Dropbox (ADR 034)
        .target(
            name: "PetaDropbox",
            dependencies: ["PetaSync", .product(name: "SwiftyDropbox", package: "SwiftyDropbox")]
        ),
        // The core behind its border, and what the app shows. No screen is in it
        .target(
            name: "PetaBorder",
            dependencies: [
                "PetaSync",
                .product(name: "PetaCore", package: "PetaCore"),
                .product(name: "PetaTypes", package: "PetaTypes"),
            ]
        ),
        .testTarget(
            name: "PetaBorderTests",
            dependencies: ["PetaBorder", .product(name: "PetaTypes", package: "PetaTypes")]
        ),
    ]
)
