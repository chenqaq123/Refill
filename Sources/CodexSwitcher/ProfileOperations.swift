import Foundation

extension ProfileStore {
    func quitCodex() throws {
        _ = try? Shell.run("/usr/bin/osascript", ["-e", "tell application \"Codex\" to quit"])

        let deadline = Date().addingTimeInterval(20)
        while Date() < deadline {
            do {
                _ = try Shell.run("/usr/bin/pgrep", ["-x", "Codex"])
                Thread.sleep(forTimeInterval: 0.4)
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

        for line in incoming.split(separator: "\n").map(String.init) where !seen.contains(line) {
            if !merged.isEmpty && !merged.hasSuffix("\n") { merged += "\n" }
            merged += line + "\n"
            seen.insert(line)
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
        for profileDir in profileDirectories() {
            try? syncWorkspaceState(from: profileDir)
        }
        let activeID = activeProfileID()
        for profileDir in profileDirectories() {
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

    func switchToProfile(_ profile: String) async throws {
        try await Task.detached(priority: .userInitiated) {
            try self.switchToProfileSync(profile)
        }.value
    }

    private func switchToProfileSync(_ profile: String) throws {
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

    func adoptCurrentAutomatically() async throws -> String {
        try await Task.detached(priority: .userInitiated) {
            try self.adoptCurrentAutomaticallySync()
        }.value
    }

    private func adoptCurrentAutomaticallySync() throws -> String {
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

    func createProvider(name: String, baseURL: String, model: String, apiKey: String) async throws -> String {
        try await Task.detached(priority: .userInitiated) {
            try self.fileManager.createDirectory(at: self.profileRoot, withIntermediateDirectories: true)
            let template = self.activeProfileID().map { self.profileURL($0) }
            let id = try self.providerStore.createProvider(
                root: self.profileRoot,
                name: name,
                baseURL: baseURL,
                model: model,
                apiKey: apiKey,
                template: template
            )
            try self.ensureSharedHistory(for: self.profileURL(id))
            try self.applySharedWorkspaceState(to: self.profileURL(id))
            return id
        }.value
    }

    func deleteProvider(_ profile: Profile) async throws {
        try await Task.detached(priority: .userInitiated) {
            try self.providerStore.deleteProvider(profile)
        }.value
    }

    func updateProvider(_ profile: Profile, name: String, baseURL: String, model: String, apiKey: String?) async throws {
        try await Task.detached(priority: .userInitiated) {
            try self.providerStore.updateProvider(
                profile,
                name: name,
                baseURL: baseURL,
                model: model,
                apiKey: apiKey
            )
        }.value
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
