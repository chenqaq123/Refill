import Foundation
import SwiftUI

@MainActor
final class AppModel: ObservableObject {
    @Published var profiles: [Profile] = []
    @Published var unmanagedCurrent: Profile?
    @Published var activeLabel = "未连接"
    @Published var notice = ""
    @Published var now = Date()
    @Published var lastSyncedAt: Date?
    @Published var busyProfileID: String?
    @Published var isWorking = false

    let store = ProfileStore()

    var officialProfiles: [Profile] {
        profiles.filter { $0.kind == .officialAccount }
    }

    var apiProfiles: [Profile] {
        profiles.filter { $0.kind == .apiProvider }
    }

    init() {
        store.reconcileSharedWorkspaceState()
        refresh()
    }

    func refresh() {
        now = Date()
        store.cacheUsageForActiveProfile()
        unmanagedCurrent = store.currentUnmanagedProfile()
        profiles = store.profiles()
        activeLabel = profiles.first(where: { $0.isActive })?.displayTitle
            ?? unmanagedCurrent?.displayTitle
            ?? "未连接"
        lastSyncedAt = now
        if notice.isEmpty { notice = "已同步" }
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
        guard busyProfileID == nil else { return }
        busyProfileID = profile.id
        isWorking = true
        notice = "正在切换 \(profile.displayTitle)"

        Task {
            do {
                try await store.switchToProfile(profile.id)
                refresh()
                notice = "已启动 \(profile.displayTitle)"
            } catch {
                notice = error.localizedDescription
            }
            busyProfileID = nil
            isWorking = false
        }
    }

    func saveCurrent() {
        guard busyProfileID == nil else { return }
        busyProfileID = "__current__"
        isWorking = true
        notice = "正在保存当前账号"

        Task {
            do {
                let profile = try await store.adoptCurrentAutomatically()
                refresh()
                notice = "当前账号已保存为 \(profile)"
            } catch {
                notice = error.localizedDescription
            }
            busyProfileID = nil
            isWorking = false
        }
    }

    func createAPIProvider(name: String, baseURL: String, model: String, apiKey: String) {
        guard busyProfileID == nil else { return }
        isWorking = true
        notice = "正在创建 API profile"

        Task {
            do {
                let id = try await store.createProvider(name: name, baseURL: baseURL, model: model, apiKey: apiKey)
                refresh()
                notice = "已创建 API profile：\(id)"
            } catch {
                notice = error.localizedDescription
            }
            isWorking = false
        }
    }

    func deleteProvider(_ profile: Profile) {
        guard profile.kind == .apiProvider, busyProfileID == nil else { return }
        busyProfileID = profile.id
        isWorking = true
        notice = "正在删除 \(profile.displayTitle)"

        Task {
            do {
                try await store.deleteProvider(profile)
                refresh()
                notice = "已删除 \(profile.displayTitle)"
            } catch {
                notice = error.localizedDescription
            }
            busyProfileID = nil
            isWorking = false
        }
    }

    func login() {
        let scriptPath = Bundle.main.resourceURL?.appendingPathComponent("codex-as").path
            ?? "/Users/cgx/Documents/Switcher/bin/codex-as"
        do {
            try store.openLoginTerminal(scriptPath: scriptPath)
            notice = "登录窗口已打开。登录完成后点同步。"
        } catch {
            notice = error.localizedDescription
        }
    }
}
