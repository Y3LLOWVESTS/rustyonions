// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "hello-ron",
    platforms: [
        .iOS(.v15),
        .macOS(.v12)
    ],
    dependencies: [
        .package(path: "../../")
    ],
    targets: [
        .executableTarget(
            name: "HelloRon",
            dependencies: [
                .product(name: "RonAppSdkSwift", package: "ron-app-sdk-swift")
            ]
        )
    ]
)

