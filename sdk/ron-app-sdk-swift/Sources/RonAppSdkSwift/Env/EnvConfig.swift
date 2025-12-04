/// RO:WHAT — Helpers for reading standard SDK env vars (gateway URL, timeouts, flags).
/// RO:WHY  — Single place to normalize env parsing across config builders.
/// RO:INTERACTS — Used by RonConfig.fromEnvironment(overrides:).
/// RO:INVARIANTS —
///   - Never throws; returns nil on malformed values.
///   - Trims whitespace; case-insensitive bool parsing.
/// RO:METRICS/LOGS —
///   - None directly; callers may log config decisions.
/// RO:CONFIG —
///   - Canonical env names live in SDK_SCHEMA_IDB.MD.
/// RO:SECURITY —
///   - Env may contain secrets (tokens); do NOT log raw values from here.
/// RO:TEST_HOOKS —
///   - Covered by RonConfigTests (env → config behavior).

import Foundation

enum EnvConfig {
    static func string(_ key: String) -> String? {
        ProcessInfo.processInfo.environment[key]
    }

    static func trimmedString(_ key: String) -> String? {
        guard let raw = string(key) else { return nil }
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    static func int(_ key: String) -> Int? {
        guard let raw = trimmedString(key) else { return nil }
        return Int(raw)
    }

    static func bool(_ key: String) -> Bool? {
        guard let raw = trimmedString(key)?.lowercased() else { return nil }
        switch raw {
        case "1", "true", "yes", "y", "on":
            return true
        case "0", "false", "no", "n", "off":
            return false
        default:
            return nil
        }
    }
}
