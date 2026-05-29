import Foundation
import SwiftUI

func parseISODate(_ text: String) -> Date? {
    let fractional = ISO8601DateFormatter()
    fractional.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    if let date = fractional.date(from: text) {
        return date
    }

    let plain = ISO8601DateFormatter()
    plain.formatOptions = [.withInternetDateTime]
    return plain.date(from: text)
}

final class Shell {
    @discardableResult
    static func run(_ executable: String, _ arguments: [String] = []) throws -> String {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = arguments

        let output = Pipe()
        let error = Pipe()
        process.standardOutput = output
        process.standardError = error

        try process.run()
        process.waitUntilExit()

        let stdout = String(data: output.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        let stderr = String(data: error.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""

        guard process.terminationStatus == 0 else {
            let message = stderr.trimmingCharacters(in: .whitespacesAndNewlines)
            throw NSError(domain: "CodexSwitcher.Shell", code: Int(process.terminationStatus), userInfo: [
                NSLocalizedDescriptionKey: message.isEmpty ? stdout : message
            ])
        }

        return stdout
    }
}

struct AccountInfo {
    var name: String?
    var email: String?
    var plan: String?
    var accountID: String?
    var subscriptionUntil: String?

    var title: String {
        if let email, !email.isEmpty { return email }
        if let name, !name.isEmpty { return name }
        if let accountID, !accountID.isEmpty { return "Account \(String(accountID.prefix(8)))" }
        return "Unknown account"
    }

    var subtitle: String {
        var parts: [String] = []
        if let name, !name.isEmpty, name != email { parts.append(name) }
        if let accountID, !accountID.isEmpty { parts.append("ID \(String(accountID.prefix(8)))") }
        return parts.isEmpty ? "ChatGPT login" : parts.joined(separator: " · ")
    }

    var planLabel: String {
        guard let plan, !plan.isEmpty else { return "Unknown" }
        return plan.prefix(1).uppercased() + plan.dropFirst()
    }

    var planDisplay: String {
        guard plan?.lowercased() == "plus" else { return planLabel }
        guard
            let subscriptionUntil,
            let date = ISO8601DateFormatter().date(from: subscriptionUntil)
        else {
            return planLabel
        }
        let formatter = DateFormatter()
        formatter.dateFormat = "MM/dd"
        return "Plus 至 \(formatter.string(from: date))"
    }

    func matches(_ other: AccountInfo) -> Bool {
        if let accountID, let otherAccountID = other.accountID, !accountID.isEmpty {
            return accountID == otherAccountID
        }
        if let email, let otherEmail = other.email, !email.isEmpty {
            return email.caseInsensitiveCompare(otherEmail) == .orderedSame
        }
        return false
    }
}

struct Profile: Identifiable {
    let id: String
    let url: URL
    let info: AccountInfo
    let isActive: Bool
    let isDesktopReady: Bool
    let usage: UsageSnapshot?
}

struct UsageWindow: Codable {
    let usedPercent: Double
    let windowMinutes: Int
    let resetsAt: TimeInterval?

    enum CodingKeys: String, CodingKey {
        case usedPercent
        case windowMinutes
        case resetsAt
        case usedPercentSnake = "used_percent"
        case windowMinutesSnake = "window_minutes"
        case resetsAtSnake = "resets_at"
    }

    init(usedPercent: Double, windowMinutes: Int, resetsAt: TimeInterval?) {
        self.usedPercent = usedPercent
        self.windowMinutes = windowMinutes
        self.resetsAt = resetsAt
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        usedPercent = try container.decodeIfPresent(Double.self, forKey: .usedPercent)
            ?? container.decode(Double.self, forKey: .usedPercentSnake)
        windowMinutes = try container.decodeIfPresent(Int.self, forKey: .windowMinutes)
            ?? container.decode(Int.self, forKey: .windowMinutesSnake)
        resetsAt = try container.decodeIfPresent(TimeInterval.self, forKey: .resetsAt)
            ?? container.decodeIfPresent(TimeInterval.self, forKey: .resetsAtSnake)
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(usedPercent, forKey: .usedPercent)
        try container.encode(windowMinutes, forKey: .windowMinutes)
        try container.encodeIfPresent(resetsAt, forKey: .resetsAt)
    }

    var label: String {
        if windowMinutes >= 10080 { return "7d" }
        if windowMinutes >= 1440 { return "\(windowMinutes / 1440)d" }
        if windowMinutes >= 60 { return "\(windowMinutes / 60)h" }
        return "\(windowMinutes)m"
    }

    var resetDate: Date? {
        resetsAt.map { Date(timeIntervalSince1970: $0) }
    }

    var remainingPercent: Double {
        min(max(100 - usedPercent, 0), 100)
    }
}

struct UsageSnapshot: Codable {
    let timestamp: String
    let primary: UsageWindow?
    let secondary: UsageWindow?

    var date: Date? {
        parseISODate(timestamp)
    }
}

struct SharedWorkspaceState: Codable {
    var savedWorkspaceRoots: [String] = []
    var projectOrder: [String] = []
    var activeWorkspaceRoots: [String] = []

    var isEmpty: Bool {
        savedWorkspaceRoots.isEmpty && projectOrder.isEmpty && activeWorkspaceRoots.isEmpty
    }
}

final class ProfileStore {
    let home = FileManager.default.homeDirectoryForCurrentUser
    let fileManager = FileManager.default

    var codexHome: URL { home.appendingPathComponent(".codex") }
    var profileRoot: URL { home.appendingPathComponent(".codex-profiles") }
    var sharedHistoryRoot: URL { profileRoot.appendingPathComponent("_shared-history") }
    var sharedSessionsURL: URL { sharedHistoryRoot.appendingPathComponent("sessions") }
    var sharedSessionIndexURL: URL { sharedHistoryRoot.appendingPathComponent("session_index.jsonl") }
    var sharedWorkspaceStateURL: URL { sharedHistoryRoot.appendingPathComponent("workspaces.json") }

    func switcherDir(for profileDir: URL) -> URL {
        profileDir.appendingPathComponent(".codex-switcher")
    }

    func usageCacheURL(for profileDir: URL) -> URL {
        switcherDir(for: profileDir).appendingPathComponent("usage.json")
    }

    func activationURL(for profileDir: URL) -> URL {
        switcherDir(for: profileDir).appendingPathComponent("activated_at.txt")
    }

    func isAccountProfileDirectory(_ url: URL) -> Bool {
        let name = url.lastPathComponent
        guard !name.hasPrefix("."), name != "_shared-history" else { return false }
        return fileManager.fileExists(atPath: url.appendingPathComponent("auth.json").path)
    }

    func profiles() -> [Profile] {
        normalizeGeneratedProfileIDs()

        guard let urls = try? fileManager.contentsOfDirectory(
            at: profileRoot,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else {
            return []
        }

        let active = activeProfileID()
        let unmanagedCurrentInfo = currentUnmanagedAccountInfo()

        return urls.compactMap { url in
            let values = try? url.resourceValues(forKeys: [.isDirectoryKey])
            guard values?.isDirectory == true else { return nil }
            guard isAccountProfileDirectory(url) else { return nil }
            let id = url.lastPathComponent
            let info = accountInfo(in: url)
            let isActive = id == active || unmanagedCurrentInfo.map { info.matches($0) } == true
            return Profile(
                id: id,
                url: url,
                info: info,
                isActive: isActive,
                isDesktopReady: isDesktopReady(url),
                usage: latestUsageSnapshot(profileID: id, dir: url)
            )
        }
        .sorted { lhs, rhs in
            if lhs.isActive != rhs.isActive { return lhs.isActive && !rhs.isActive }
            return lhs.info.title.localizedCaseInsensitiveCompare(rhs.info.title) == .orderedAscending
        }
    }

    func normalizeGeneratedProfileIDs() {
        guard let urls = try? fileManager.contentsOfDirectory(
            at: profileRoot,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else {
            return
        }

        let active = activeProfileID()
        for url in urls {
            let id = url.lastPathComponent
            guard isAccountProfileDirectory(url) else { continue }
            guard id.hasPrefix("login-") else { continue }
            let info = accountInfo(in: url)
            guard info.email != nil || info.name != nil || info.accountID != nil else { continue }
            let newID = suggestedProfileID(for: info)
            guard newID != id else { continue }
            let destination = profileURL(newID)
            guard !fileManager.fileExists(atPath: destination.path) else { continue }

            do {
                try fileManager.moveItem(at: url, to: destination)
                if active == id, isCodexHomeSymlink() {
                    try fileManager.removeItem(at: codexHome)
                    try fileManager.createSymbolicLink(at: codexHome, withDestinationURL: destination)
                }
            } catch {
                continue
            }
        }
    }

    func currentUnmanagedProfile() -> Profile? {
        guard fileManager.fileExists(atPath: codexHome.path), !isCodexHomeSymlink() else { return nil }
        if let currentInfo = currentUnmanagedAccountInfo(), profileMatching(currentInfo) != nil { return nil }
        return Profile(
            id: "__current__",
            url: codexHome,
            info: accountInfo(in: codexHome),
            isActive: true,
            isDesktopReady: isDesktopReady(codexHome),
            usage: latestUsageSnapshot(profileID: "__current__", dir: codexHome)
        )
    }

    func currentUnmanagedAccountInfo() -> AccountInfo? {
        guard fileManager.fileExists(atPath: codexHome.path), !isCodexHomeSymlink() else { return nil }
        return accountInfo(in: codexHome)
    }

    func profileMatching(_ info: AccountInfo) -> Profile? {
        guard let urls = try? fileManager.contentsOfDirectory(
            at: profileRoot,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else {
            return nil
        }

        for url in urls {
            let values = try? url.resourceValues(forKeys: [.isDirectoryKey])
            guard values?.isDirectory == true else { continue }
            guard isAccountProfileDirectory(url) else { continue }
            let id = url.lastPathComponent
            let profileInfo = accountInfo(in: url)
            if profileInfo.matches(info) {
                return Profile(
                    id: id,
                    url: url,
                    info: profileInfo,
                    isActive: true,
                    isDesktopReady: isDesktopReady(url),
                    usage: latestUsageSnapshot(profileID: id, dir: url)
                )
            }
        }
        return nil
    }

    func activeProfileID() -> String? {
        guard let destination = try? fileManager.destinationOfSymbolicLink(atPath: codexHome.path) else {
            return nil
        }

        let resolved = URL(fileURLWithPath: destination, relativeTo: codexHome.deletingLastPathComponent()).standardizedFileURL
        let root = profileRoot.standardizedFileURL.path + "/"
        guard resolved.path.hasPrefix(root) else { return nil }
        return String(resolved.path.dropFirst(root.count))
    }

    func isCodexHomeSymlink() -> Bool {
        guard let values = try? codexHome.resourceValues(forKeys: [.isSymbolicLinkKey]) else {
            return false
        }
        return values.isSymbolicLink == true
    }

    func profileURL(_ profile: String) -> URL {
        profileRoot.appendingPathComponent(profile)
    }

    func isDesktopReady(_ dir: URL) -> Bool {
        fileManager.fileExists(atPath: dir.appendingPathComponent("auth.json").path)
            && fileManager.fileExists(atPath: dir.appendingPathComponent("config.toml").path)
            && fileManager.fileExists(atPath: dir.appendingPathComponent("computer-use").path)
            && fileManager.fileExists(atPath: dir.appendingPathComponent(".codex-global-state.json").path)
    }

    func accountInfo(in dir: URL) -> AccountInfo {
        let authURL = dir.appendingPathComponent("auth.json")
        guard
            let data = try? Data(contentsOf: authURL),
            let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            return AccountInfo()
        }

        let tokens = root["tokens"] as? [String: Any]
        let idPayload = decodeJWTPayload(tokens?["id_token"] as? String)
        let accessPayload = decodeJWTPayload(tokens?["access_token"] as? String)
        let authClaims = (idPayload?["https://api.openai.com/auth"] as? [String: Any])
            ?? (accessPayload?["https://api.openai.com/auth"] as? [String: Any])
        let profileClaims = accessPayload?["https://api.openai.com/profile"] as? [String: Any]

        return AccountInfo(
            name: firstString([idPayload?["name"], accessPayload?["name"]]),
            email: firstString([idPayload?["email"], profileClaims?["email"]]),
            plan: firstString([authClaims?["chatgpt_plan_type"], authClaims?["plan_type"]]),
            accountID: firstString([tokens?["account_id"], authClaims?["chatgpt_account_id"], authClaims?["account_id"]]),
            subscriptionUntil: firstString([authClaims?["chatgpt_subscription_active_until"]])
        )
    }

    func latestUsageSnapshot(profileID: String, dir: URL) -> UsageSnapshot? {
        if profileID == activeProfileID(), let live = activeUsageSnapshotFromSharedHistory() {
            try? writeUsageCache(live, for: dir)
            return live
        }

        if let cached = readUsageCache(for: dir) {
            return cached
        }

        // One-time seed from the pre-shared backup for accounts that had local sessions
        // before history sharing was enabled.
        let backups = sharedHistoryRoot.appendingPathComponent("backups")
        var roots: [URL] = []
        if let urls = try? fileManager.contentsOfDirectory(
            at: backups,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) {
            roots.append(contentsOf: urls.filter { $0.lastPathComponent.hasPrefix("\(profileID)-sessions-") })
        }

        var latest: UsageSnapshot?
        for root in roots {
            for snapshot in usageSnapshots(in: root) {
                if latest == nil || snapshot.timestamp > (latest?.timestamp ?? "") {
                    latest = snapshot
                }
            }
        }
        if let latest {
            try? writeUsageCache(latest, for: dir)
        }
        return latest
    }

    func readUsageCache(for profileDir: URL) -> UsageSnapshot? {
        let url = usageCacheURL(for: profileDir)
        guard
            let data = try? Data(contentsOf: url),
            let snapshot = try? JSONDecoder().decode(UsageSnapshot.self, from: data)
        else {
            return nil
        }
        return snapshot
    }

    func writeUsageCache(_ snapshot: UsageSnapshot, for profileDir: URL) throws {
        let dir = switcherDir(for: profileDir)
        try fileManager.createDirectory(at: dir, withIntermediateDirectories: true)
        let data = try JSONEncoder().encode(snapshot)
        try data.write(to: usageCacheURL(for: profileDir), options: [.atomic])
    }

    func writeActivationDate(for profileDir: URL, date: Date = Date()) throws {
        let dir = switcherDir(for: profileDir)
        try fileManager.createDirectory(at: dir, withIntermediateDirectories: true)
        try ISO8601DateFormatter().string(from: date).write(to: activationURL(for: profileDir), atomically: true, encoding: .utf8)
    }

    func readActivationDate(for profileDir: URL) -> Date? {
        guard
            let text = try? String(contentsOf: activationURL(for: profileDir), encoding: .utf8)
                .trimmingCharacters(in: .whitespacesAndNewlines)
        else {
            return nil
        }
        return parseISODate(text)
    }

    func activeUsageSnapshotFromSharedHistory() -> UsageSnapshot? {
        guard let activeID = activeProfileID() else { return nil }
        let activatedAt = readActivationDate(for: profileURL(activeID))
        return usageSnapshots(in: sharedSessionsURL)
            .filter { snapshot in
                guard let activatedAt else { return false }
                guard let snapshotDate = snapshot.date else { return false }
                return snapshotDate >= activatedAt
            }
            .max { $0.timestamp < $1.timestamp }
    }

    func cacheUsageForActiveProfile() {
        guard
            let activeID = activeProfileID(),
            let snapshot = activeUsageSnapshotFromSharedHistory()
        else {
            return
        }
        try? writeUsageCache(snapshot, for: profileURL(activeID))
    }

    func usageSnapshots(in root: URL) -> [UsageSnapshot] {
        guard let enumerator = fileManager.enumerator(at: root, includingPropertiesForKeys: nil) else {
            return []
        }

        var snapshots: [UsageSnapshot] = []
        for case let url as URL in enumerator where url.pathExtension == "jsonl" {
            guard let content = try? String(contentsOf: url, encoding: .utf8) else { continue }
            for line in content.split(separator: "\n") where line.contains("\"rate_limits\"") {
                if let snapshot = usageSnapshot(from: String(line)) {
                    snapshots.append(snapshot)
                }
            }
        }
        return snapshots
    }

    func usageSnapshot(from line: String) -> UsageSnapshot? {
        guard
            let data = line.data(using: .utf8),
            let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            let timestamp = root["timestamp"] as? String,
            let payload = root["payload"] as? [String: Any],
            let rateLimits = payload["rate_limits"] as? [String: Any]
        else {
            return nil
        }

        return UsageSnapshot(
            timestamp: timestamp,
            primary: usageWindow(from: rateLimits["primary"]),
            secondary: usageWindow(from: rateLimits["secondary"])
        )
    }

    func usageWindow(from value: Any?) -> UsageWindow? {
        guard let dict = value as? [String: Any] else { return nil }
        let used = dict["used_percent"] as? Double ?? (dict["used_percent"] as? Int).map(Double.init)
        let window = dict["window_minutes"] as? Int
        guard let used, let window else { return nil }
        let resets = dict["resets_at"] as? Double ?? (dict["resets_at"] as? Int).map(Double.init)
        return UsageWindow(usedPercent: used, windowMinutes: window, resetsAt: resets)
    }

    func firstString(_ values: [Any?]) -> String? {
        for value in values {
            if let string = value as? String, !string.isEmpty { return string }
        }
        return nil
    }

    func decodeJWTPayload(_ token: String?) -> [String: Any]? {
        guard let token else { return nil }
        let parts = token.split(separator: ".")
        guard parts.count >= 2 else { return nil }

        var payload = String(parts[1])
            .replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        while payload.count % 4 != 0 { payload.append("=") }

        guard
            let data = Data(base64Encoded: payload),
            let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            return nil
        }
        return object
    }

    func suggestedProfileID(for info: AccountInfo) -> String {
        let base = info.email ?? info.name ?? info.accountID ?? "codex-account"
        let lowered = base.lowercased()
        let allowed = lowered.map { character -> Character in
            if character.isLetter || character.isNumber { return character }
            if character == "@" || character == "." || character == "_" || character == "-" { return "-" }
            return "-"
        }
        var slug = String(allowed)
        while slug.contains("--") { slug = slug.replacingOccurrences(of: "--", with: "-") }
        slug = slug.trimmingCharacters(in: CharacterSet(charactersIn: "-"))
        if slug.isEmpty { slug = "codex-account" }
        if let plan = info.plan, !plan.isEmpty { slug += "-\(plan.lowercased())" }

        var candidate = slug
        var index = 2
        while fileManager.fileExists(atPath: profileURL(candidate).path) {
            candidate = "\(slug)-\(index)"
            index += 1
        }
        return candidate
    }

    func quitCodex() throws {
        _ = try? Shell.run("/usr/bin/osascript", ["-e", "tell application \"Codex\" to quit"])

        let deadline = Date().addingTimeInterval(20)
        while Date() < deadline {
            do {
                _ = try Shell.run("/usr/bin/pgrep", ["-x", "Codex"])
                Thread.sleep(forTimeInterval: 1)
            } catch {
                return
            }
        }

        throw NSError(domain: "CodexSwitcher.ProfileStore", code: 1, userInfo: [
            NSLocalizedDescriptionKey: "Codex 仍在运行。请手动退出 Codex 后再试。"
        ])
    }

    func launchCodex() throws {
        try Shell.run("/usr/bin/open", ["-a", "Codex"])
    }

    func copyIfMissing(_ name: String, from source: URL, to target: URL) throws {
        let sourceURL = source.appendingPathComponent(name)
        let targetURL = target.appendingPathComponent(name)
        guard fileManager.fileExists(atPath: sourceURL.path), !fileManager.fileExists(atPath: targetURL.path) else {
            return
        }
        try fileManager.copyItem(at: sourceURL, to: targetURL)
    }

    func isSymlink(_ url: URL) -> Bool {
        (try? url.resourceValues(forKeys: [.isSymbolicLinkKey]).isSymbolicLink) == true
    }

    func copyDirectoryContentsIfMissing(from source: URL, to target: URL) throws {
        guard fileManager.fileExists(atPath: source.path) else { return }
        try fileManager.createDirectory(at: target, withIntermediateDirectories: true)

        guard let enumerator = fileManager.enumerator(at: source, includingPropertiesForKeys: [.isDirectoryKey]) else {
            return
        }

        for case let sourceURL as URL in enumerator {
            let relativePath = String(sourceURL.path.dropFirst(source.path.count)).trimmingCharacters(in: CharacterSet(charactersIn: "/"))
            guard !relativePath.isEmpty else { continue }
            let targetURL = target.appendingPathComponent(relativePath)
            if fileManager.fileExists(atPath: targetURL.path) { continue }

            let values = try? sourceURL.resourceValues(forKeys: [.isDirectoryKey])
            if values?.isDirectory == true {
                try fileManager.createDirectory(at: targetURL, withIntermediateDirectories: true)
            } else {
                try fileManager.createDirectory(at: targetURL.deletingLastPathComponent(), withIntermediateDirectories: true)
                try fileManager.copyItem(at: sourceURL, to: targetURL)
            }
        }
    }

    func appendSessionIndexIfNeeded(from source: URL) throws {
        guard fileManager.fileExists(atPath: source.path) else { return }
        let existing = (try? String(contentsOf: sharedSessionIndexURL, encoding: .utf8)) ?? ""
        let incoming = (try? String(contentsOf: source, encoding: .utf8)) ?? ""
        var seen = Set(existing.split(separator: "\n").map(String.init))
        var merged = existing

        for line in incoming.split(separator: "\n").map(String.init) {
            if !seen.contains(line) {
                if !merged.isEmpty && !merged.hasSuffix("\n") { merged += "\n" }
                merged += line + "\n"
                seen.insert(line)
            }
        }

        try fileManager.createDirectory(at: sharedHistoryRoot, withIntermediateDirectories: true)
        try merged.write(to: sharedSessionIndexURL, atomically: true, encoding: .utf8)
    }

    func backupAndRemove(_ url: URL, label: String) throws {
        guard fileManager.fileExists(atPath: url.path) || isSymlink(url) else { return }
        let backups = sharedHistoryRoot.appendingPathComponent("backups")
        try fileManager.createDirectory(at: backups, withIntermediateDirectories: true)
        let backup = backups.appendingPathComponent("\(label)-\(Int(Date().timeIntervalSince1970))")
        try fileManager.moveItem(at: url, to: backup)
    }

    func ensureSharedHistory(for profileDir: URL) throws {
        try fileManager.createDirectory(at: sharedSessionsURL, withIntermediateDirectories: true)

        let localSessions = profileDir.appendingPathComponent("sessions")
        if !isSymlink(localSessions) {
            try copyDirectoryContentsIfMissing(from: localSessions, to: sharedSessionsURL)
            try backupAndRemove(localSessions, label: profileDir.lastPathComponent + "-sessions")
            try fileManager.createSymbolicLink(at: localSessions, withDestinationURL: sharedSessionsURL)
        }

        let localIndex = profileDir.appendingPathComponent("session_index.jsonl")
        if !isSymlink(localIndex) {
            try appendSessionIndexIfNeeded(from: localIndex)
            try backupAndRemove(localIndex, label: profileDir.lastPathComponent + "-session-index")
            try fileManager.createSymbolicLink(at: localIndex, withDestinationURL: sharedSessionIndexURL)
        } else if !fileManager.fileExists(atPath: sharedSessionIndexURL.path) {
            try "".write(to: sharedSessionIndexURL, atomically: true, encoding: .utf8)
        }
    }

    func accountProfileDirectories() -> [URL] {
        guard let urls = try? fileManager.contentsOfDirectory(
            at: profileRoot,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else {
            return []
        }

        return urls.filter { url in
            let values = try? url.resourceValues(forKeys: [.isDirectoryKey])
            return values?.isDirectory == true && isAccountProfileDirectory(url)
        }
    }

    func readGlobalState(in profileDir: URL) -> [String: Any]? {
        let url = profileDir.appendingPathComponent(".codex-global-state.json")
        guard
            let data = try? Data(contentsOf: url),
            let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            return nil
        }
        return object
    }

    func writeGlobalState(_ state: [String: Any], to profileDir: URL) throws {
        let url = profileDir.appendingPathComponent(".codex-global-state.json")
        let data = try JSONSerialization.data(withJSONObject: state, options: [.prettyPrinted, .sortedKeys])
        try data.write(to: url, options: [.atomic])
    }

    func stringArray(_ value: Any?) -> [String] {
        value as? [String] ?? []
    }

    func unique(_ values: [String]) -> [String] {
        var seen = Set<String>()
        var output: [String] = []
        for value in values where !value.isEmpty && !seen.contains(value) {
            seen.insert(value)
            output.append(value)
        }
        return output
    }

    func workspaceState(in profileDir: URL) -> SharedWorkspaceState {
        guard let state = readGlobalState(in: profileDir) else {
            return SharedWorkspaceState()
        }
        return SharedWorkspaceState(
            savedWorkspaceRoots: stringArray(state["electron-saved-workspace-roots"]),
            projectOrder: stringArray(state["project-order"]),
            activeWorkspaceRoots: stringArray(state["active-workspace-roots"])
        )
    }

    func readSharedWorkspaceState() -> SharedWorkspaceState {
        guard
            let data = try? Data(contentsOf: sharedWorkspaceStateURL),
            let state = try? JSONDecoder().decode(SharedWorkspaceState.self, from: data)
        else {
            return SharedWorkspaceState()
        }
        return state
    }

    func writeSharedWorkspaceState(_ state: SharedWorkspaceState) throws {
        try fileManager.createDirectory(at: sharedHistoryRoot, withIntermediateDirectories: true)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        try encoder.encode(state).write(to: sharedWorkspaceStateURL, options: [.atomic])
    }

    func mergedWorkspaceState(preferred: SharedWorkspaceState, fallback: SharedWorkspaceState) -> SharedWorkspaceState {
        let saved = unique(preferred.savedWorkspaceRoots + fallback.savedWorkspaceRoots + preferred.projectOrder + fallback.projectOrder)
        let order = unique(preferred.projectOrder + fallback.projectOrder + saved)
        let active = preferred.activeWorkspaceRoots.isEmpty ? fallback.activeWorkspaceRoots : preferred.activeWorkspaceRoots
        return SharedWorkspaceState(savedWorkspaceRoots: saved, projectOrder: order, activeWorkspaceRoots: active)
    }

    func syncWorkspaceState(from profileDir: URL) throws {
        let local = workspaceState(in: profileDir)
        guard !local.isEmpty else { return }
        let merged = mergedWorkspaceState(preferred: local, fallback: readSharedWorkspaceState())
        try writeSharedWorkspaceState(merged)
    }

    func applySharedWorkspaceState(to profileDir: URL) throws {
        let shared = readSharedWorkspaceState()
        guard !shared.isEmpty, var globalState = readGlobalState(in: profileDir) else {
            return
        }

        let local = workspaceState(in: profileDir)
        let merged = mergedWorkspaceState(preferred: shared, fallback: local)
        globalState["electron-saved-workspace-roots"] = merged.savedWorkspaceRoots
        globalState["project-order"] = merged.projectOrder
        if !merged.activeWorkspaceRoots.isEmpty {
            globalState["active-workspace-roots"] = merged.activeWorkspaceRoots
        }
        try writeGlobalState(globalState, to: profileDir)
    }

    func syncWorkspaceStateForActiveProfile() {
        guard let activeID = activeProfileID() else { return }
        try? syncWorkspaceState(from: profileURL(activeID))
    }

    func reconcileSharedWorkspaceState() {
        for profileDir in accountProfileDirectories() {
            try? syncWorkspaceState(from: profileDir)
        }
        let activeID = activeProfileID()
        for profileDir in accountProfileDirectories() {
            if profileDir.lastPathComponent == activeID { continue }
            try? applySharedWorkspaceState(to: profileDir)
        }
    }

    func hydrateDesktopProfile(_ profile: String) throws {
        let target = profileURL(profile)
        guard fileManager.fileExists(atPath: target.path) else {
            throw NSError(domain: "CodexSwitcher.ProfileStore", code: 8, userInfo: [
                NSLocalizedDescriptionKey: "profile 不存在：\(profile)"
            ])
        }

        if isCodexHomeSymlink(), activeProfileID() == profile { return }
        guard fileManager.fileExists(atPath: codexHome.path) else {
            throw NSError(domain: "CodexSwitcher.ProfileStore", code: 9, userInfo: [
                NSLocalizedDescriptionKey: "找不到可用于初始化 Desktop profile 的 ~/.codex。"
            ])
        }

        let supportItems = [
            ".codex-global-state.json",
            ".codex-global-state.json.bak",
            ".personality_migration",
            ".tmp",
            "ambient-suggestions",
            "cache",
            "computer-use",
            "installation_id",
            "memories",
            "models_cache.json",
            "plugins",
            "rules",
            "skills",
            "sqlite",
            "tmp",
            "vendor_imports",
            "version.json"
        ]

        try fileManager.createDirectory(at: target, withIntermediateDirectories: true)
        for item in supportItems {
            try copyIfMissing(item, from: codexHome, to: target)
        }

        for folder in ["sessions", "log", "shell_snapshots"] {
            let url = target.appendingPathComponent(folder)
            if !fileManager.fileExists(atPath: url.path) {
                try fileManager.createDirectory(at: url, withIntermediateDirectories: true)
            }
        }

        try ensureSharedHistory(for: target)
        try applySharedWorkspaceState(to: target)
    }

    func switchToProfile(_ profile: String) throws {
        let target = profileURL(profile)
        guard fileManager.fileExists(atPath: target.path) else {
            throw NSError(domain: "CodexSwitcher.ProfileStore", code: 2, userInfo: [
                NSLocalizedDescriptionKey: "profile 不存在：\(profile)"
            ])
        }

        cacheUsageForActiveProfile()
        try quitCodex()
        syncWorkspaceStateForActiveProfile()
        try hydrateDesktopProfile(profile)
        try ensureSharedHistory(for: target)
        try applySharedWorkspaceState(to: target)

        if isCodexHomeSymlink() {
            try fileManager.removeItem(at: codexHome)
        } else if fileManager.fileExists(atPath: codexHome.path) {
            throw NSError(domain: "CodexSwitcher.ProfileStore", code: 3, userInfo: [
                NSLocalizedDescriptionKey: "当前账号还没保存成 profile。请先点“保存”。"
            ])
        }

        try fileManager.createSymbolicLink(at: codexHome, withDestinationURL: target)
        try writeActivationDate(for: target)
        try launchCodex()
    }

    @discardableResult
    func adoptCurrentAutomatically() throws -> String {
        guard fileManager.fileExists(atPath: codexHome.path) else {
            throw NSError(domain: "CodexSwitcher.ProfileStore", code: 5, userInfo: [
                NSLocalizedDescriptionKey: "~/.codex 不存在。"
            ])
        }
        guard !isCodexHomeSymlink() else {
            throw NSError(domain: "CodexSwitcher.ProfileStore", code: 6, userInfo: [
                NSLocalizedDescriptionKey: "当前账号已经保存为 profile。"
            ])
        }

        let profile = suggestedProfileID(for: accountInfo(in: codexHome))
        let target = profileURL(profile)

        try quitCodex()
        try fileManager.createDirectory(at: profileRoot, withIntermediateDirectories: true)
        try fileManager.moveItem(at: codexHome, to: target)
        try ensureSharedHistory(for: target)
        try syncWorkspaceState(from: target)
        try applySharedWorkspaceState(to: target)
        try writeActivationDate(for: target)
        try fileManager.createSymbolicLink(at: codexHome, withDestinationURL: target)
        try launchCodex()
        return profile
    }

    func openLoginTerminal(scriptPath: String) throws {
        let generatedProfile = "login-\(Int(Date().timeIntervalSince1970))"
        let escapedScript = scriptPath.replacingOccurrences(of: "'", with: "'\\''")
        let command = "'\(escapedScript)' --login '\(generatedProfile)'"
        let appleScript = """
        tell application "Terminal"
          activate
          do script "\(command.replacingOccurrences(of: "\"", with: "\\\""))"
        end tell
        """
        try Shell.run("/usr/bin/osascript", ["-e", appleScript])
    }
}

@MainActor
final class AppModel: ObservableObject {
    @Published var profiles: [Profile] = []
    @Published var unmanagedCurrent: Profile?
    @Published var activeLabel = "None"
    @Published var notice = ""
    @Published var now = Date()
    @Published var lastSyncedAt: Date?

    let store = ProfileStore()

    init() {
        store.reconcileSharedWorkspaceState()
        refresh()
    }

    func refresh() {
        now = Date()
        store.cacheUsageForActiveProfile()
        unmanagedCurrent = store.currentUnmanagedProfile()
        profiles = store.profiles()
        activeLabel = profiles.first(where: { $0.isActive })?.info.title
            ?? unmanagedCurrent?.info.title
            ?? "未连接"
        lastSyncedAt = now
        notice = "已同步"
    }

    func tick() {
        now = Date()
    }

    func autoRefreshIfNeeded() {
        let shouldRefresh = lastSyncedAt.map { Date().timeIntervalSince($0) >= 60 } ?? true
        if shouldRefresh {
            refresh()
        }
    }

    func switchTo(_ profile: Profile) {
        do {
            try store.switchToProfile(profile.id)
            refresh()
            notice = "已启动 \(profile.info.title)"
        } catch {
            notice = error.localizedDescription
        }
    }

    func saveCurrent() {
        do {
            let profile = try store.adoptCurrentAutomatically()
            refresh()
            notice = "当前账号已保存为 \(profile)"
        } catch {
            notice = error.localizedDescription
        }
    }

    func login() {
        let scriptPath = Bundle.main.resourceURL?.appendingPathComponent("codex-as").path
            ?? "/Users/cgx/Documents/Switcher/bin/codex-as"
        do {
            try store.openLoginTerminal(scriptPath: scriptPath)
            notice = "登录窗口已打开。登录完成后点刷新。"
        } catch {
            notice = error.localizedDescription
        }
    }
}

struct CodexSwitcherAppView: View {
    @StateObject private var model = AppModel()
    private let ticker = Timer.publish(every: 1, on: .main, in: .common).autoconnect()

    var body: some View {
        ZStack {
            AppDesign.appBackground
                .ignoresSafeArea()

            ScrollView {
                VStack(spacing: 14) {
                    HeaderView(model: model)

                    if let current = model.unmanagedCurrent {
                        VStack(alignment: .leading, spacing: 8) {
                            SectionTitle(title: "待保存", count: 1)
                            AccountRow(profile: current, unmanaged: true, now: model.now) {
                                model.saveCurrent()
                            }
                        }
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        SectionTitle(title: "账号", count: model.profiles.count)

                        LazyVStack(spacing: 10) {
                            ForEach(model.profiles) { profile in
                                AccountRow(profile: profile, unmanaged: false, now: model.now) {
                                    model.switchTo(profile)
                                }
                            }
                        }
                    }

                    if model.profiles.isEmpty && model.unmanagedCurrent == nil {
                        EmptyStateView()
                    }
                }
                .frame(maxWidth: AppDesign.pageMaxWidth)
                .padding(.horizontal, 20)
                .padding(.vertical, 16)
                .frame(maxWidth: .infinity)
            }
        }
        .frame(minWidth: 760, minHeight: 500)
        .onReceive(ticker) { _ in
            model.tick()
            model.autoRefreshIfNeeded()
        }
    }
}

struct HeaderView: View {
    @ObservedObject var model: AppModel
    @State private var isHovering = false

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .center, spacing: 12) {
                ZStack {
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(AppDesign.blue.gradient)
                    Image(systemName: "switch.2")
                        .font(.system(size: 18, weight: .bold))
                        .foregroundStyle(.white)
                }
                .frame(width: 40, height: 40)
                .shadow(color: AppDesign.blue.opacity(0.18), radius: 8, x: 0, y: 5)

                VStack(alignment: .leading, spacing: 3) {
                    Text("Codex Switcher")
                        .font(.system(size: 22, weight: .bold))
                        .foregroundStyle(.primary)
                        .textSelection(.enabled)

                    Text(model.activeLabel)
                        .font(.system(size: 12.5, weight: .semibold))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .textSelection(.enabled)
                }

                Spacer()

                ToolbarActionButton(title: "登录", systemImage: "plus", tint: AppDesign.teal) {
                    model.login()
                }

                ToolbarActionButton(title: "同步", systemImage: "arrow.clockwise", tint: AppDesign.blue) {
                    model.refresh()
                }
            }

            HStack(spacing: 8) {
                StatusChip(title: "当前账号", systemImage: "checkmark.circle.fill", tint: AppDesign.green, filled: true)
                StatusChip(title: "共享会话", systemImage: "text.bubble.fill", tint: AppDesign.teal)
                StatusChip(title: "\(model.profiles.count) 个账号", systemImage: "person.2.fill", tint: AppDesign.blue)
                StatusChip(title: "1 分钟同步", systemImage: "clock.arrow.circlepath", tint: .secondary)

                if let lastSyncedAt = model.lastSyncedAt {
                    Text("同步 \(relativeText(for: lastSyncedAt, now: model.now))")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(.tertiary)
                        .lineLimit(1)
                }

                Spacer()

                if !model.notice.isEmpty {
                    Label(model.notice, systemImage: "sparkle")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .textSelection(.enabled)
                }
            }
        }
        .padding(14)
        .cardSurface(isActive: false, isHovering: isHovering)
        .onHover { isHovering = $0 }
    }

    func relativeText(for date: Date, now: Date) -> String {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .short
        return formatter.localizedString(for: date, relativeTo: now)
    }
}

struct SectionTitle: View {
    let title: String
    let count: Int

    var body: some View {
        HStack(spacing: 8) {
            Text(title)
                .font(.system(size: 12.5, weight: .bold))
                .foregroundStyle(.secondary)
            Text("\(count)")
                .font(.system(size: 11, weight: .bold, design: .rounded))
                .foregroundStyle(.tertiary)
                .padding(.horizontal, 7)
                .padding(.vertical, 3)
                .background(Capsule(style: .continuous).fill(Color.primary.opacity(0.07)))
            Spacer()
        }
        .padding(.horizontal, 4)
    }
}

struct AccountRow: View {
    let profile: Profile
    let unmanaged: Bool
    let now: Date
    let action: () -> Void
    @State private var isHovering = false

    var body: some View {
        HStack(spacing: 12) {
            Avatar(profile: profile)

            VStack(alignment: .leading, spacing: 7) {
                HStack(spacing: 8) {
                    Text(profile.info.title)
                        .font(.system(size: 15, weight: .bold))
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .textSelection(.enabled)

                    if profile.isActive {
                        Image(systemName: "checkmark.seal.fill")
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(AppDesign.green)
                    }
                }

                Text(profile.info.subtitle)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .textSelection(.enabled)

                HStack(spacing: 7) {
                    if profile.isActive {
                        Pill("当前", color: AppDesign.green, filled: true)
                    }
                    Pill(profile.info.planDisplay, color: AppDesign.blue)
                    Pill(profile.isDesktopReady ? "可启动" : "待补齐", color: profile.isDesktopReady ? AppDesign.teal : AppDesign.orange)
                    if unmanaged {
                        Pill("未保存", color: .gray)
                    }
                }
            }
            .frame(minWidth: 200, maxWidth: .infinity, alignment: .leading)

            Spacer(minLength: 8)

            UsageBadge(usage: profile.usage, now: now)

            LaunchButton(title: unmanaged ? "保存" : (profile.isActive ? "重启" : "启动"), active: profile.isActive, action: action)
        }
        .padding(.vertical, 12)
        .padding(.horizontal, 14)
        .cardSurface(isActive: profile.isActive, isHovering: isHovering)
        .scaleEffect(isHovering ? 1.006 : 1)
        .animation(.easeOut(duration: 0.16), value: isHovering)
        .onHover { hovering in
            isHovering = hovering
        }
        .accessibilityElement(children: .contain)
    }
}

struct UsageBadge: View {
    let usage: UsageSnapshot?
    let now: Date

    var body: some View {
        VStack(alignment: .trailing, spacing: 6) {
            if let usage {
                HStack(spacing: 8) {
                    if let primary = usage.primary {
                        UsageLine(window: primary)
                    }
                    if let secondary = usage.secondary {
                        UsageLine(window: secondary)
                    }
                }
                Text("更新 \(relativeText(for: usage.date))")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
            } else {
                StatusChip(title: "待刷新", systemImage: "hourglass", tint: .secondary)
                Text("运行一次后显示额度")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
            }
        }
        .frame(width: 188, alignment: .trailing)
    }

    func relativeText(for date: Date?) -> String {
        guard let date else { return "未知" }
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .short
        return formatter.localizedString(for: date, relativeTo: now)
    }
}

struct UsageLine: View {
    let window: UsageWindow

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(alignment: .firstTextBaseline, spacing: 5) {
                Text(window.label)
                    .font(.system(size: 11.5, weight: .bold, design: .rounded))
                    .foregroundStyle(.secondary)
                    .frame(width: 22, alignment: .leading)
                Text("\(Int(window.remainingPercent.rounded()))%")
                    .font(.system(size: 13.5, weight: .bold, design: .rounded))
                    .foregroundStyle(tint)
                    .frame(width: 42, alignment: .trailing)
            }

            MiniUsageBar(value: window.remainingPercent, tint: tint)
                .frame(width: 66, height: 5)

            Text(resetText)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(.tertiary)
                .lineLimit(1)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .background {
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .fill(tint.opacity(0.085))
        }
    }

    var tint: Color {
        if window.remainingPercent <= 15 { return AppDesign.red }
        if window.remainingPercent <= 35 { return AppDesign.orange }
        return AppDesign.blue
    }

    var resetText: String {
        guard let date = window.resetDate else { return "重置时间未知" }
        let relative = RelativeDateTimeFormatter()
        relative.unitsStyle = .abbreviated
        let absolute = DateFormatter()
        absolute.dateFormat = "HH:mm"
        return "\(relative.localizedString(for: date, relativeTo: Date())) · \(absolute.string(from: date))"
    }
}

struct MiniUsageBar: View {
    let value: Double
    let tint: Color

    var body: some View {
        GeometryReader { proxy in
            ZStack(alignment: .leading) {
                Capsule(style: .continuous)
                    .fill(Color.primary.opacity(0.09))
                Capsule(style: .continuous)
                    .fill(tint)
                    .frame(width: max(4, proxy.size.width * CGFloat(min(max(value, 0), 100) / 100)))
            }
        }
    }
}

struct LaunchButton: View {
    let title: String
    let active: Bool
    let action: () -> Void
    @State private var isHovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Image(systemName: active ? "arrow.clockwise" : "play.fill")
                    .font(.system(size: 12, weight: .bold))
                Text(title)
                    .font(.system(size: 13, weight: .bold))
            }
            .foregroundStyle(.white)
            .frame(minWidth: 72)
            .padding(.vertical, 8)
            .padding(.horizontal, 11)
            .background {
                RoundedRectangle(cornerRadius: AppDesign.buttonRadius, style: .continuous)
                    .fill(buttonTint.gradient)
            }
            .shadow(color: buttonTint.opacity(isHovering ? 0.28 : 0.14), radius: isHovering ? 12 : 6, x: 0, y: isHovering ? 5 : 3)
        }
        .buttonStyle(.plain)
        .scaleEffect(isHovering ? 1.035 : 1)
        .animation(.easeOut(duration: 0.14), value: isHovering)
        .onHover { hovering in
            isHovering = hovering
        }
    }

    var buttonTint: Color { active ? AppDesign.teal : AppDesign.blue }
}

struct Avatar: View {
    let profile: Profile

    var body: some View {
        ZStack {
            Circle()
                .fill(avatarGradient)
                .shadow(color: profile.isActive ? AppDesign.blue.opacity(0.20) : .black.opacity(0.06), radius: 10, x: 0, y: 5)
            Text(initial)
                .font(.system(size: 18, weight: .black))
                .foregroundStyle(.white)
        }
        .frame(width: 40, height: 40)
    }

    var initial: String {
        let source = profile.info.email ?? profile.info.name ?? profile.id
        return String(source.prefix(1)).uppercased()
    }

    var avatarGradient: LinearGradient {
        if profile.isActive {
            return LinearGradient(colors: [AppDesign.blue, AppDesign.teal], startPoint: .topLeading, endPoint: .bottomTrailing)
        }
        let source = profile.info.email ?? profile.info.name ?? profile.id
        let palette: [[Color]] = [
            [Color(red: 0.36, green: 0.45, blue: 0.95), Color(red: 0.05, green: 0.57, blue: 0.73)],
            [Color(red: 0.85, green: 0.35, blue: 0.30), Color(red: 0.90, green: 0.62, blue: 0.22)],
            [Color(red: 0.25, green: 0.58, blue: 0.36), Color(red: 0.03, green: 0.55, blue: 0.52)]
        ]
        let index = abs(source.hashValue) % palette.count
        return LinearGradient(colors: palette[index], startPoint: .topLeading, endPoint: .bottomTrailing)
    }
}

struct Pill: View {
    let text: String
    let color: Color
    let filled: Bool

    init(_ text: String, color: Color, filled: Bool = false) {
        self.text = text
        self.color = color
        self.filled = filled
    }

    var body: some View {
        Text(text)
            .font(.system(size: 10.5, weight: .bold))
            .lineLimit(1)
            .foregroundStyle(filled ? .white : color)
            .padding(.horizontal, 7)
            .padding(.vertical, 3)
            .background {
                Capsule().fill(filled ? color : color.opacity(0.13))
            }
    }
}

struct EmptyStateView: View {
    var body: some View {
        VStack(spacing: 10) {
            Image(systemName: "person.crop.circle.badge.plus")
                .font(.system(size: 42, weight: .semibold))
                .foregroundStyle(AppDesign.blue)
            Text("还没有账号")
                .font(.system(size: 18, weight: .bold))
            Text("点右上角登录，登录完成后刷新。")
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
        }
        .frame(maxWidth: .infinity, minHeight: 260)
        .cardSurface()
    }
}

@main
struct CodexAccountSwitcherApp: App {
    var body: some Scene {
        WindowGroup {
            CodexSwitcherAppView()
                .background(.regularMaterial)
        }
        .windowStyle(.titleBar)
    }
}
