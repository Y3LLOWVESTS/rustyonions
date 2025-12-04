// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ron-app-sdk-swift",
    platforms: [
        .iOS(.v15),
        .macOS(.v12),
        .tvOS(.v15),
        .watchOS(.v8)
    ],
    products: [
        .library(
            name: "RonAppSdkSwift",
            targets: ["RonAppSdkSwift"]
        ),
    ],
    dependencies: [
        // Add SwiftLog or other dependencies here when needed.
        // .package(url: "https://github.com/apple/swift-log.git", from: "1.0.0"),
    ],
    targets: [
        .target(
            name: "RonAppSdkSwift",
            path: "Sources/RonAppSdkSwift"
        ),
        .testTarget(
            name: "RonAppSdkSwiftTests",
            dependencies: ["RonAppSdkSwift"],
            path: "Tests/RonAppSdkSwiftTests"
        ),
    ]
)

