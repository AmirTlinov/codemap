// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "SwiftFixture",
    products: [
        .library(name: "SwiftFixture", targets: ["SwiftFixture"]),
    ],
    targets: [
        .target(name: "SwiftFixture", path: "Sources/SwiftFixture"),
        .testTarget(name: "SwiftFixtureTests", dependencies: ["SwiftFixture"]),
    ]
)
