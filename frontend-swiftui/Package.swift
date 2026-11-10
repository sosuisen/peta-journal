// swift-tools-version:5.9
import PackageDescription

// The packages under generated/ are made from the Rust crates by generate.sh.
let package = Package(
    name: "PetaBorder",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(name: "PetaBorder", targets: ["PetaBorder"])
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
        .testTarget(name: "PetaBorderTests", dependencies: ["PetaBorder"]),
    ]
)
