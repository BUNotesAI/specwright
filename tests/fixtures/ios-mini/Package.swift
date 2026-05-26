// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "IosMini",
    platforms: [
        .iOS(.v17)
    ],
    products: [
        .library(name: "IosMini", targets: ["IosMini"])
    ],
    targets: [
        .target(name: "IosMini"),
        .testTarget(name: "IosMiniTests", dependencies: ["IosMini"])
    ]
)
