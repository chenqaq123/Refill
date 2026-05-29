import Foundation

final class ProfileStore: @unchecked Sendable {
    let home = FileManager.default.homeDirectoryForCurrentUser
    let fileManager = FileManager.default
    let providerStore = ProviderStore()

    var codexHome: URL { home.appendingPathComponent(".codex") }
    var profileRoot: URL { home.appendingPathComponent(".codex-profiles") }
    var sharedHistoryRoot: URL { profileRoot.appendingPathComponent("_shared-history") }
    var sharedSessionsURL: URL { sharedHistoryRoot.appendingPathComponent("sessions") }
    var sharedSessionIndexURL: URL { sharedHistoryRoot.appendingPathComponent("session_index.jsonl") }
    var sharedDesktopStateRoot: URL { sharedHistoryRoot.appendingPathComponent("desktop-state") }
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

    func isProfileDirectory(_ url: URL) -> Bool {
        let name = url.lastPathComponent
        guard !name.hasPrefix("."), name != "_shared-history" else { return false }
        return fileManager.fileExists(atPath: url.appendingPathComponent("auth.json").path)
            || providerStore.isProviderProfile(url)
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
            guard values?.isDirectory == true, isProfileDirectory(url) else { return nil }
            let id = url.lastPathComponent
            let provider = providerStore.readProvider(in: url)
            let info = provider.map(providerInfo) ?? accountInfo(in: url)
            let isActive = id == active || (provider == nil && unmanagedCurrentInfo.map { info.matches($0) } == true)
            return Profile(
                id: id,
                url: url,
                info: info,
                kind: provider == nil ? .officialAccount : .apiProvider,
                isActive: isActive,
                isDesktopReady: isDesktopReady(url),
                usage: provider == nil ? latestUsageSnapshot(profileID: id, dir: url) : nil,
                provider: provider
            )
        }
        .sorted { lhs, rhs in
            if lhs.isActive != rhs.isActive { return lhs.isActive && !rhs.isActive }
            if lhs.kind != rhs.kind { return lhs.kind == .officialAccount }
            return lhs.displayTitle.localizedCaseInsensitiveCompare(rhs.displayTitle) == .orderedAscending
        }
    }

    func providerInfo(_ provider: APIProviderConfig) -> AccountInfo {
        AccountInfo(
            name: provider.title,
            email: nil,
            plan: "api",
            accountID: provider.providerID,
            subscriptionUntil: nil
        )
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
            guard fileManager.fileExists(atPath: url.appendingPathComponent("auth.json").path) else { continue }
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
            kind: .officialAccount,
            isActive: true,
            isDesktopReady: isDesktopReady(codexHome),
            usage: latestUsageSnapshot(profileID: "__current__", dir: codexHome),
            provider: nil
        )
    }

    func currentUnmanagedAccountInfo() -> AccountInfo? {
        guard fileManager.fileExists(atPath: codexHome.path), !isCodexHomeSymlink() else { return nil }
        return accountInfo(in: codexHome)
    }

    func profileMatching(_ info: AccountInfo) -> Profile? {
        for url in profileDirectories() where fileManager.fileExists(atPath: url.appendingPathComponent("auth.json").path) {
            let id = url.lastPathComponent
            let profileInfo = accountInfo(in: url)
            if profileInfo.matches(info) {
                return Profile(
                    id: id,
                    url: url,
                    info: profileInfo,
                    kind: .officialAccount,
                    isActive: true,
                    isDesktopReady: isDesktopReady(url),
                    usage: latestUsageSnapshot(profileID: id, dir: url),
                    provider: nil
                )
            }
        }
        return nil
    }

    func profileDirectories() -> [URL] {
        guard let urls = try? fileManager.contentsOfDirectory(
            at: profileRoot,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else {
            return []
        }
        return urls.filter { url in
            let values = try? url.resourceValues(forKeys: [.isDirectoryKey])
            return values?.isDirectory == true && isProfileDirectory(url)
        }
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
        if let provider = providerStore.readProvider(in: dir) {
            return fileManager.fileExists(atPath: dir.appendingPathComponent("config.toml").path)
                && fileManager.fileExists(atPath: dir.appendingPathComponent(".codex-global-state.json").path)
                && providerStore.keyExists(config: provider)
        }
        return fileManager.fileExists(atPath: dir.appendingPathComponent("auth.json").path)
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
            providerStore.readProvider(in: profileURL(activeID)) == nil,
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
}
