import SwiftUI

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
