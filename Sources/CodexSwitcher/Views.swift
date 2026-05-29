import SwiftUI

struct CodexSwitcherAppView: View {
    @StateObject private var model = AppModel()
    @State private var showingAPISheet = false
    private let ticker = Timer.publish(every: 1, on: .main, in: .common).autoconnect()

    var body: some View {
        ZStack {
            AppDesign.appBackground
                .ignoresSafeArea()

            ScrollView {
                VStack(spacing: 14) {
                    HeaderView(model: model, showingAPISheet: $showingAPISheet)

                    if let current = model.unmanagedCurrent {
                        VStack(alignment: .leading, spacing: 8) {
                            SectionTitle(title: "待保存", count: 1)
                            AccountRow(profile: current, unmanaged: true, now: model.now, isBusy: model.busyProfileID == current.id) {
                                model.saveCurrent()
                            }
                        }
                    }

                    ProfileSection(
                        title: "官方账号",
                        profiles: model.officialProfiles,
                        now: model.now,
                        busyProfileID: model.busyProfileID,
                        onLaunch: model.switchTo(_:),
                        onDelete: nil
                    )

                    ProfileSection(
                        title: "API",
                        profiles: model.apiProfiles,
                        now: model.now,
                        busyProfileID: model.busyProfileID,
                        onLaunch: model.switchTo(_:),
                        onDelete: model.deleteProvider(_:)
                    )

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
        .frame(minWidth: 780, minHeight: 520)
        .sheet(isPresented: $showingAPISheet) {
            APIProviderSheet { name, baseURL, modelName, apiKey in
                showingAPISheet = false
                model.createAPIProvider(name: name, baseURL: baseURL, model: modelName, apiKey: apiKey)
            }
        }
        .onReceive(ticker) { _ in
            model.tick()
            model.autoRefreshIfNeeded()
        }
    }
}

struct HeaderView: View {
    @ObservedObject var model: AppModel
    @Binding var showingAPISheet: Bool
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

                ToolbarActionButton(title: "API", systemImage: "key.fill", tint: AppDesign.orange) {
                    showingAPISheet = true
                }

                ToolbarActionButton(title: "同步", systemImage: "arrow.clockwise", tint: AppDesign.blue) {
                    model.refresh()
                }
            }

            HStack(spacing: 8) {
                StatusChip(title: "当前账号", systemImage: "checkmark.circle.fill", tint: AppDesign.green, filled: true)
                StatusChip(title: "共享会话", systemImage: "text.bubble.fill", tint: AppDesign.teal)
                StatusChip(title: "\(model.officialProfiles.count) 个账号", systemImage: "person.2.fill", tint: AppDesign.blue)
                StatusChip(title: "\(model.apiProfiles.count) 个 API", systemImage: "network", tint: AppDesign.orange)
                StatusChip(title: "后台切换", systemImage: "bolt.horizontal", tint: .secondary)

                if let lastSyncedAt = model.lastSyncedAt {
                    Text("同步 \(relativeText(for: lastSyncedAt, now: model.now))")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(.tertiary)
                        .lineLimit(1)
                }

                Spacer()

                if !model.notice.isEmpty {
                    Label(model.notice, systemImage: model.isWorking ? "hourglass" : "sparkle")
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

struct ProfileSection: View {
    let title: String
    let profiles: [Profile]
    let now: Date
    let busyProfileID: String?
    let onLaunch: (Profile) -> Void
    let onDelete: ((Profile) -> Void)?

    var body: some View {
        if !profiles.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                SectionTitle(title: title, count: profiles.count)
                LazyVStack(spacing: 10) {
                    ForEach(profiles) { profile in
                        AccountRow(
                            profile: profile,
                            unmanaged: false,
                            now: now,
                            isBusy: busyProfileID == profile.id,
                            onDelete: onDelete.map { delete in { delete(profile) } },
                            action: { onLaunch(profile) }
                        )
                    }
                }
            }
        }
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
    let isBusy: Bool
    var onDelete: (() -> Void)?
    let action: () -> Void
    @State private var isHovering = false

    init(
        profile: Profile,
        unmanaged: Bool,
        now: Date,
        isBusy: Bool,
        onDelete: (() -> Void)? = nil,
        action: @escaping () -> Void
    ) {
        self.profile = profile
        self.unmanaged = unmanaged
        self.now = now
        self.isBusy = isBusy
        self.onDelete = onDelete
        self.action = action
    }

    var body: some View {
        HStack(spacing: 12) {
            Avatar(profile: profile)

            VStack(alignment: .leading, spacing: 7) {
                HStack(spacing: 8) {
                    Text(profile.displayTitle)
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

                Text(profile.displaySubtitle)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .textSelection(.enabled)

                HStack(spacing: 7) {
                    if profile.isActive {
                        Pill("当前", color: AppDesign.green, filled: true)
                    }
                    Pill(profile.primaryPill, color: profile.kind == .apiProvider ? AppDesign.orange : AppDesign.blue)
                    Pill(profile.isDesktopReady ? "可启动" : "待补齐", color: profile.isDesktopReady ? AppDesign.teal : AppDesign.orange)
                    if unmanaged {
                        Pill("未保存", color: .gray)
                    }
                    if profile.kind == .apiProvider {
                        Pill("Keychain", color: AppDesign.teal)
                    }
                }
            }
            .frame(minWidth: 210, maxWidth: .infinity, alignment: .leading)

            Spacer(minLength: 8)

            if profile.kind == .apiProvider {
                ProviderBadge(profile: profile)
            } else {
                UsageBadge(usage: profile.usage, now: now)
            }

            if let onDelete, profile.kind == .apiProvider, !profile.isActive {
                IconButton(systemImage: "trash", tint: .secondary, action: onDelete)
            }

            LaunchButton(title: buttonTitle, active: profile.isActive, isBusy: isBusy, action: action)
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

    var buttonTitle: String {
        if isBusy { return "切换中" }
        if unmanaged { return "保存" }
        return profile.isActive ? "重启" : "启动"
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
                        UsageLine(window: primary, now: now)
                    }
                    if let secondary = usage.secondary {
                        UsageLine(window: secondary, now: now)
                    }
                }
                Text(footerText(for: usage))
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
        .frame(width: 198, alignment: .trailing)
    }

    func footerText(for usage: UsageSnapshot) -> String {
        let prefix = usage.hasEstimatedReset(now: now) ? "含本地估算" : "真实同步"
        return "\(prefix) \(relativeText(for: usage.date))"
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
    let now: Date

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(alignment: .firstTextBaseline, spacing: 5) {
                Text(window.label)
                    .font(.system(size: 11.5, weight: .bold, design: .rounded))
                    .foregroundStyle(.secondary)
                    .frame(width: 22, alignment: .leading)
                Text("\(Int(value.rounded()))%")
                    .font(.system(size: 13.5, weight: .bold, design: .rounded))
                    .foregroundStyle(tint)
                    .frame(width: 42, alignment: .trailing)
            }

            MiniUsageBar(value: value, tint: tint)
                .frame(width: 66, height: 5)

            Text(window.displayResetText(now: now))
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

    var value: Double {
        window.effectiveRemainingPercent(now: now)
    }

    var tint: Color {
        if window.isReset(now: now) { return AppDesign.green }
        if value <= 15 { return AppDesign.red }
        if value <= 35 { return AppDesign.orange }
        return AppDesign.blue
    }
}

struct ProviderBadge: View {
    let profile: Profile

    var body: some View {
        VStack(alignment: .trailing, spacing: 6) {
            StatusChip(title: profile.provider?.model ?? "model", systemImage: "cpu", tint: AppDesign.orange)
            Text(URL(string: profile.provider?.baseURL ?? "")?.host ?? "Responses API")
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(.tertiary)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .frame(width: 156, alignment: .trailing)
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
    let isBusy: Bool
    let action: () -> Void
    @State private var isHovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Image(systemName: isBusy ? "hourglass" : (active ? "arrow.clockwise" : "play.fill"))
                    .font(.system(size: 12, weight: .bold))
                Text(title)
                    .font(.system(size: 13, weight: .bold))
            }
            .foregroundStyle(.white)
            .frame(minWidth: 78)
            .padding(.vertical, 8)
            .padding(.horizontal, 11)
            .background {
                RoundedRectangle(cornerRadius: AppDesign.buttonRadius, style: .continuous)
                    .fill(buttonTint.gradient)
            }
            .shadow(color: buttonTint.opacity(isHovering ? 0.28 : 0.14), radius: isHovering ? 12 : 6, x: 0, y: isHovering ? 5 : 3)
        }
        .buttonStyle(.plain)
        .disabled(isBusy)
        .scaleEffect(isHovering ? 1.035 : 1)
        .animation(.easeOut(duration: 0.14), value: isHovering)
        .onHover { hovering in
            isHovering = hovering
        }
    }

    var buttonTint: Color {
        if isBusy { return .secondary }
        return active ? AppDesign.teal : AppDesign.blue
    }
}

struct IconButton: View {
    let systemImage: String
    let tint: Color
    let action: () -> Void
    @State private var isHovering = false

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: 12, weight: .bold))
                .foregroundStyle(tint)
                .frame(width: 28, height: 28)
                .background(Circle().fill(Color.primary.opacity(isHovering ? 0.10 : 0.05)))
        }
        .buttonStyle(.plain)
        .onHover { isHovering = $0 }
    }
}

struct Avatar: View {
    let profile: Profile

    var body: some View {
        ZStack {
            Circle()
                .fill(avatarGradient)
                .shadow(color: profile.isActive ? AppDesign.blue.opacity(0.20) : .black.opacity(0.06), radius: 10, x: 0, y: 5)
            if profile.kind == .apiProvider {
                Image(systemName: "key.fill")
                    .font(.system(size: 14, weight: .black))
                    .foregroundStyle(.white)
            } else {
                Text(initial)
                    .font(.system(size: 18, weight: .black))
                    .foregroundStyle(.white)
            }
        }
        .frame(width: 40, height: 40)
    }

    var initial: String {
        let source = profile.info.email ?? profile.info.name ?? profile.id
        return String(source.prefix(1)).uppercased()
    }

    var avatarGradient: LinearGradient {
        if profile.kind == .apiProvider {
            return LinearGradient(colors: [AppDesign.orange, AppDesign.teal], startPoint: .topLeading, endPoint: .bottomTrailing)
        }
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
            Text("点右上角登录，登录完成后同步。")
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
        }
        .frame(maxWidth: .infinity, minHeight: 260)
        .cardSurface()
    }
}

struct APIProviderSheet: View {
    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var baseURL = ""
    @State private var model = ""
    @State private var apiKey = ""
    let onSave: (String, String, String, String) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("添加 API")
                .font(.system(size: 20, weight: .bold))

            TextField("名称，例如 OpenRouter", text: $name)
                .textFieldStyle(.roundedBorder)
            TextField("Base URL，例如 https://api.example.com/v1", text: $baseURL)
                .textFieldStyle(.roundedBorder)
            TextField("Model，例如 gpt-5.5-compatible", text: $model)
                .textFieldStyle(.roundedBorder)
            SecureField("API Key", text: $apiKey)
                .textFieldStyle(.roundedBorder)

            HStack {
                Spacer()
                Button("取消") {
                    dismiss()
                }
                .keyboardShortcut(.cancelAction)

                Button("保存") {
                    onSave(name, baseURL, model, apiKey)
                }
                .keyboardShortcut(.defaultAction)
                .disabled(!canSave)
            }
        }
        .padding(22)
        .frame(width: 440)
    }

    var canSave: Bool {
        let url = URL(string: baseURL.trimmingCharacters(in: .whitespacesAndNewlines))
        return !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && (url?.scheme == "http" || url?.scheme == "https")
            && url?.host != nil
            && !model.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !apiKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }
}
