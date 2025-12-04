// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "vapor-facet-demo",
    platforms: [
        .iOS(.v15),
        .macOS(.v12)
    ],
    dependencies: [
        // .package(url: "https://github.com/vapor/vapor.git", from: "4.0.0"),
        .package(path: "../../")
    ],
    targets: [
        .executableTarget(
            name: "App",
            dependencies: [
                .product(name: "RonAppSdkSwift", package: "ron-app-sdk-swift")
                // , .product(name: "Vapor", package: "vapor")
            ]
        )
    ]
)

