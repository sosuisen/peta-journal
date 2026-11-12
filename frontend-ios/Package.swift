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
        .library(name: "PetaSync", targets: ["PetaSync"])
    ],
    targets: [
        // The sync with the cloud. It knows neither the core nor Dropbox
        .target(name: "PetaSync"),
        .testTarget(name: "PetaSyncTests", dependencies: ["PetaSync"]),
    ]
)
