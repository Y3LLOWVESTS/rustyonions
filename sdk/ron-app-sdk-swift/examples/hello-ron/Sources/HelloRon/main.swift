import Foundation
import RonAppSdkSwift

struct Greeting: Decodable {
    let message: String
}

@main
struct HelloRon {
    static func main() async {
        do {
            // Prefer env var for base URL in dev:
            //   RON_SDK_GATEWAY_ADDR=https://my-node.example.com
            // For local http:// use RON_SDK_INSECURE_HTTP=1 and never ship that config.
            let overrides = RonConfigOverrides(
                allowInsecureHttp: true, // local dev only
                debug: true
            )

            let config = try RonConfig.fromEnvironment(overrides: overrides)
            let client = try RonClient(config: config)

            let res: AppResponse<Greeting> = await client.get("/hello")

            if res.ok, let value = res.value {
                print("Server says:", value.message)
            } else if let problem = res.problem {
                print("Call failed:", problem.code, "-", problem.message)
            } else {
                print("Call failed with unknown error")
            }
        } catch {
            // SDK-local errors: misconfiguration, invalid URL, etc.
            print("RON SDK error:", error)
        }
    }
}
