// swift-tools-version:5.9
import PackageDescription

// The packages under generated/ are made from the Rust crates by generate.sh.
let package = Package(
    name: "PetaBorder",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(name: "PetaBorder", targets: ["PetaBorder"]),
        .library(name: "PetaScreens", targets: ["PetaScreens"]),
    ],
    dependencies: [
        .package(path: "generated/PetaCore"),
        .package(path: "generated/swift/PetaTypes"),
    ],
    targets: [
        .target(
            name: "PetaBorder",
            dependencies: [
                .product(name: "PetaCore", package: "PetaCore"),
                .product(name: "PetaTypes", package: "PetaTypes"),
            ]
        ),
        // The views. They are a library so that a test can put them in a window
        .target(
            name: "PetaScreens",
            dependencies: [
                "PetaBorder",
                .product(name: "PetaTypes", package: "PetaTypes"),
            ]
        ),
        .testTarget(name: "PetaBorderTests", dependencies: ["PetaBorder", "PetaScreens"]),
    ]
)
