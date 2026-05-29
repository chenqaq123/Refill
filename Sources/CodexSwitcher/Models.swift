import Foundation

enum ProfileKind: String, Codable {
    case officialAccount
    case apiProvider
}

struct APIProviderConfig: Codable, Equatable {
    var id: String
    var name: String
    var baseURL: String
    var model: String
    var providerID: String
    var createdAt: String

    var keychainService: String {
        "local.codex.account-switcher.\(providerID)"
    }

    var title: String {
        name.isEmpty ? providerID : name
    }

    var subtitle: String {
        let host = URL(string: baseURL)?.host ?? baseURL
        return "\(model) · \(host)"
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
        guard let subscriptionUntil, let date = ISO8601DateFormatter().date(from: subscriptionUntil) else {
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
    let kind: ProfileKind
    let isActive: Bool
    let isDesktopReady: Bool
    let usage: UsageSnapshot?
    let provider: APIProviderConfig?

    var displayTitle: String {
        provider?.title ?? info.title
    }

    var displaySubtitle: String {
        provider?.subtitle ?? info.subtitle
    }

    var primaryPill: String {
        provider == nil ? info.planDisplay : "Responses API"
    }
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

    func isReset(now: Date) -> Bool {
        guard let resetDate else { return false }
        return resetDate <= now
    }

    func effectiveRemainingPercent(now: Date) -> Double {
        isReset(now: now) ? 100 : remainingPercent
    }

    func displayResetText(now: Date) -> String {
        guard let resetDate else { return "重置时间未知" }
        if resetDate <= now { return "预计已恢复" }
        let relative = RelativeDateTimeFormatter()
        relative.unitsStyle = .abbreviated
        let absolute = DateFormatter()
        absolute.dateFormat = "HH:mm"
        return "\(relative.localizedString(for: resetDate, relativeTo: now)) · \(absolute.string(from: resetDate))"
    }
}

struct UsageSnapshot: Codable {
    let timestamp: String
    let primary: UsageWindow?
    let secondary: UsageWindow?

    var date: Date? {
        parseISODate(timestamp)
    }

    func hasEstimatedReset(now: Date) -> Bool {
        [primary, secondary].compactMap { $0 }.contains { $0.isReset(now: now) }
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
