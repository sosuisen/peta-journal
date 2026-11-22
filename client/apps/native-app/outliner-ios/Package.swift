// swift-tools-version:5.9
import PackageDescription

// The packages under generated/ and ../swift-shared/generated/ are made from the Rust crates by
// generate.sh.
let package = Package(
    name: "PetaJournal",
    platforms: [
        .iOS(.v17),
        // The parts without a screen are tested on the Mac, with no simulator
        .macOS(.v14),
    ],
    products: [
        .library(name: "PetaBorder", targets: ["PetaBorder"]),
        .library(name: "PetaDropbox", targets: ["PetaDropbox"]),
    ],
    dependencies: [
        .package(path: "generated/PetaCore"),
        // The types of the border, from the package that both apps share
        .package(path: "../swift-shared/generated/swift/PetaTypes"),
        .package(path: "../swift-shared"),
        .package(url: "https://github.com/dropbox/SwiftyDropbox.git", exact: "10.2.4"),
    ],
    targets: [
        // The account of Dropbox and the calls of its SDK that the sync of the core makes
        // (ADR 034, ADR 054)
        .target(
            name: "PetaDropbox",
            dependencies: ["PetaBorder", .product(name: "SwiftyDropbox", package: "SwiftyDropbox")]
        ),
        .testTarget(name: "PetaDropboxTests", dependencies: ["PetaDropbox"]),
        // The core behind its border, the sync with the cloud, and what the app shows. No
        // screen is in it
        .target(
            name: "PetaBorder",
            dependencies: [
                .product(name: "PetaCore", package: "PetaCore"),
                .product(name: "PetaTypes", package: "PetaTypes"),
                .product(name: "PetaShared", package: "swift-shared"),
            ]
        ),
        .testTarget(
            name: "PetaBorderTests",
            dependencies: ["PetaBorder", .product(name: "PetaTypes", package: "PetaTypes")]
        ),
    ]
)
