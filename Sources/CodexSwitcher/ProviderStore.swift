import Foundation

final class ProviderStore: @unchecked Sendable {
    let fileManager = FileManager.default

    func providerConfigURL(for profileDir: URL) -> URL {
        profileDir.appendingPathComponent(".codex-switcher/provider.json")
    }

    func readProvider(in profileDir: URL) -> APIProviderConfig? {
        let url = providerConfigURL(for: profileDir)
        guard
            let data = try? Data(contentsOf: url),
            let config = try? JSONDecoder().decode(APIProviderConfig.self, from: data)
        else {
            return nil
        }
        return config
    }

    func isProviderProfile(_ profileDir: URL) -> Bool {
        readProvider(in: profileDir) != nil
    }

    func createProvider(root: URL, name: String, baseURL: String, model: String, apiKey: String, template: URL?) throws -> String {
        let id = uniqueProviderID(root: root, name: name)
        let providerID = "switcher-\(id)"
        let profileDir = root.appendingPathComponent(id)
        let config = APIProviderConfig(
            id: id,
            name: name.trimmingCharacters(in: .whitespacesAndNewlines),
            baseURL: normalizedBaseURL(baseURL),
            model: model.trimmingCharacters(in: .whitespacesAndNewlines),
            providerID: providerID,
            createdAt: ISO8601DateFormatter().string(from: Date())
        )

        try fileManager.createDirectory(at: profileDir.appendingPathComponent(".codex-switcher"), withIntermediateDirectories: true)
        try writeKey(apiKey, config: config)
        try writeProvider(config, to: profileDir)
        try writeCodexConfig(config, to: profileDir, template: template)
        try createDesktopPlaceholders(in: profileDir)
        return id
    }

    func deleteProvider(_ profile: Profile) throws {
        guard let provider = profile.provider else { return }
        try? deleteKey(config: provider)
        try fileManager.removeItem(at: profile.url)
    }

    func keyExists(config: APIProviderConfig) -> Bool {
        (try? Shell.run("/usr/bin/security", ["find-generic-password", "-w", "-s", config.keychainService])) != nil
    }

    private func writeProvider(_ config: APIProviderConfig, to profileDir: URL) throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        try encoder.encode(config).write(to: providerConfigURL(for: profileDir), options: [.atomic])
    }

    private func writeKey(_ apiKey: String, config: APIProviderConfig) throws {
        let key = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !key.isEmpty else {
            throw NSError(domain: "CodexSwitcher.ProviderStore", code: 1, userInfo: [
                NSLocalizedDescriptionKey: "API key 不能为空。"
            ])
        }
        try Shell.run("/usr/bin/security", [
            "add-generic-password",
            "-U",
            "-s", config.keychainService,
            "-a", config.providerID,
            "-w", key
        ])
    }

    private func deleteKey(config: APIProviderConfig) throws {
        try Shell.run("/usr/bin/security", [
            "delete-generic-password",
            "-s", config.keychainService
        ])
    }

    private func writeCodexConfig(_ config: APIProviderConfig, to profileDir: URL, template: URL?) throws {
        var projectConfig = ""
        if let template {
            projectConfig = extractProjectConfig(from: template.appendingPathComponent("config.toml"))
        }
        let text = """
        model_provider = "\(tomlEscape(config.providerID))"
        model = "\(tomlEscape(config.model))"
        model_reasoning_effort = "medium"

        [model_providers.\(tomlBareKey(config.providerID))]
        name = "\(tomlEscape(config.title))"
        base_url = "\(tomlEscape(config.baseURL))"
        wire_api = "responses"
        requires_openai_auth = false

        [model_providers.\(tomlBareKey(config.providerID)).auth]
        command = "/usr/bin/security"
        args = ["find-generic-password", "-w", "-s", "\(tomlEscape(config.keychainService))"]

        \(projectConfig)
        """
        try text.write(to: profileDir.appendingPathComponent("config.toml"), atomically: true, encoding: .utf8)
    }

    private func createDesktopPlaceholders(in profileDir: URL) throws {
        for folder in ["sessions", "log", "shell_snapshots", "tmp"] {
            try fileManager.createDirectory(at: profileDir.appendingPathComponent(folder), withIntermediateDirectories: true)
        }
        let globalState = profileDir.appendingPathComponent(".codex-global-state.json")
        if !fileManager.fileExists(atPath: globalState.path) {
            try "{}".write(to: globalState, atomically: true, encoding: .utf8)
        }
    }

    private func extractProjectConfig(from url: URL) -> String {
        guard let content = try? String(contentsOf: url, encoding: .utf8) else { return "" }
        let lines = content.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        var keep: [String] = []
        var copying = false
        for line in lines {
            if line.hasPrefix("[projects.") {
                copying = true
            } else if copying && line.hasPrefix("[") && !line.hasPrefix("[projects.") {
                copying = false
            }
            if copying {
                keep.append(line)
            }
        }
        return keep.isEmpty ? "" : keep.joined(separator: "\n")
    }

    private func uniqueProviderID(root: URL, name: String) -> String {
        let slug = slugify(name.isEmpty ? "api-provider" : name)
        var candidate = slug
        var index = 2
        while fileManager.fileExists(atPath: root.appendingPathComponent(candidate).path) {
            candidate = "\(slug)-\(index)"
            index += 1
        }
        return candidate
    }

    private func slugify(_ text: String) -> String {
        let allowed = text.lowercased().map { character -> Character in
            if character.isLetter || character.isNumber { return character }
            if character == "@" || character == "." || character == "_" || character == "-" { return "-" }
            return "-"
        }
        var slug = String(allowed)
        while slug.contains("--") { slug = slug.replacingOccurrences(of: "--", with: "-") }
        slug = slug.trimmingCharacters(in: CharacterSet(charactersIn: "-"))
        return slug.isEmpty ? "api-provider" : slug
    }

    private func normalizedBaseURL(_ value: String) -> String {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.hasSuffix("/") ? String(trimmed.dropLast()) : trimmed
    }

    private func tomlEscape(_ value: String) -> String {
        value
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
    }

    private func tomlBareKey(_ value: String) -> String {
        if value.range(of: #"^[A-Za-z0-9_-]+$"#, options: .regularExpression) != nil {
            return value
        }
        return "\"\(tomlEscape(value))\""
    }
}
