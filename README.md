# Codex Account Switcher

This repo keeps multiple Codex logins separated by profile. It does not copy tokens into scripts. Each profile gets its own `CODEX_HOME` directory under `~/.codex-profiles`.

## Refill v3 Tauri Preview

v3 is a parallel Tauri desktop console named **Refill**. It keeps the SwiftUI v2 app intact, but adds a modern React/Tailwind UI with a Rust backend for profile scanning, API provider management, usage display, shared history diagnostics, and background switching progress.

Build the v3 app:

```sh
npm install
npm run tauri:build
```

The generated app and DMG are:

```sh
/Users/cgx/Documents/Switcher/src-tauri/target/release/bundle/macos/Refill.app
/Users/cgx/Documents/Switcher/src-tauri/target/release/bundle/dmg/Refill_3.0.4_aarch64.dmg
```

v3 uses the same profile layout as v2:

- `~/.codex-profiles`
- `.codex-switcher/provider.json`
- Keychain service `local.codex.account-switcher.<providerID>`
- `_shared-history/sessions`
- `_shared-history/session_index.jsonl`
- `_shared-history/desktop-state`

The v3 UI is a desktop console: official accounts and API providers are grouped into cards, switching emits progress instead of blocking the UI, and the detail panel exposes diagnostics without showing sensitive keys.

### v3.0.3 Fixes

- Repairs shared `sessions`, `session_index.jsonl`, and Desktop sqlite state across every saved profile after Codex exits during a switch, so newly added API profiles keep the same chat history.
- Rebuilds missing official-account usage caches from that account's activation window or its backups without overwriting existing caches.

### v3.0.4 Fixes

- Aligns shared thread `model_provider` to the target official account or API provider during switching, so OpenRouter/DeepSeek can see the same left-sidebar project chat history.

## Mac App

Build the double-clickable app:

```sh
/Users/cgx/Documents/Switcher/build.sh
```

Then open:

```sh
open "/Users/cgx/Documents/Switcher/dist/Codex Account Switcher.app"
```

The app can:

- list profiles in `~/.codex-profiles`
- show each account as a card with email, plan, readiness, and current-account status
- show effective remaining usage; expired 5h/7d windows are locally treated as recovered until the next real Codex snapshot arrives
- save the current `~/.codex` automatically using account info, without asking for a profile name
- switch profiles with one click by quitting Codex, hydrating missing Desktop support files, relinking `~/.codex`, and reopening Codex in a background task
- add Responses-compatible third-party API profiles whose API keys are stored in macOS Keychain
- open Terminal for first-time login to a new profile; generated login profiles are renamed from account info after refresh

## Setup

Make the wrapper available in your shell:

```sh
chmod +x /Users/cgx/Documents/Switcher/bin/codex-as
```

Optional convenience alias:

```sh
alias codex-as=/Users/cgx/Documents/Switcher/bin/codex-as
```

## First-Time Login

Log in once per account:

```sh
/Users/cgx/Documents/Switcher/bin/codex-as --login work
/Users/cgx/Documents/Switcher/bin/codex-as --login personal
```

Then check them:

```sh
/Users/cgx/Documents/Switcher/bin/codex-as --status work
/Users/cgx/Documents/Switcher/bin/codex-as --status personal
```

## Daily Use

Start Codex with a chosen account:

```sh
/Users/cgx/Documents/Switcher/bin/codex-as work
/Users/cgx/Documents/Switcher/bin/codex-as personal -C /Users/cgx/Documents/Switcher
```

Run non-interactive commands:

```sh
/Users/cgx/Documents/Switcher/bin/codex-as work exec "review this repo"
```

List profiles:

```sh
/Users/cgx/Documents/Switcher/bin/codex-as --list
```

## Desktop App Note

The CLI respects `CODEX_HOME`, so this profile approach is clean for terminal use.

For Desktop, use `bin/codex-desktop-as`. It switches the whole `~/.codex` directory, so each account keeps its own auth, config, sessions, logs, and local state.

First save your current Desktop state into a profile from the Mac app. The app derives the profile name from the signed-in account email and plan.

The legacy CLI can still adopt with an explicit name:

```sh
chmod +x /Users/cgx/Documents/Switcher/bin/codex-desktop-as
/Users/cgx/Documents/Switcher/bin/codex-desktop-as --adopt-current main
```

Then switch Desktop accounts:

```sh
/Users/cgx/Documents/Switcher/bin/codex-desktop-as work
/Users/cgx/Documents/Switcher/bin/codex-desktop-as personal
/Users/cgx/Documents/Switcher/bin/codex-desktop-as main
```

The script quits Codex, waits for it to exit, points `~/.codex` at the selected profile, then launches Codex again.

If a profile was created by CLI login only, switching now hydrates it first. Hydration copies Desktop support files such as `computer-use`, plugins, caches, and UI state when they are missing, but it does not overwrite that profile's `auth.json`, `config.toml`, sessions, or logs.

## Usage Display

Codex only emits quota details while an account is actively used, so inactive accounts use their last cached `rate_limits` snapshot. The app now applies the reset timestamp locally:

- if a 5h or 7d window has not reset yet, the card shows the remaining percentage from the cached snapshot
- if the reset timestamp is in the past, the card shows that window as `100%` with `预计已恢复`
- the footer still shows when the last real Codex snapshot was captured, so estimated recovery is not confused with a fresh server sync

The next time the account is launched and Codex emits new `rate_limits`, that real snapshot replaces the estimate.

## Third-Party API Profiles

Click `API` in the app to add a Responses-compatible provider. Each provider gets a profile under `~/.codex-profiles` with a generated `config.toml` like:

```toml
model_provider = "switcher-provider-name"
model = "your-model"

[model_providers.switcher-provider-name]
name = "Provider Name"
base_url = "https://provider.example.com/v1"
wire_api = "responses"
requires_openai_auth = false

[model_providers.switcher-provider-name.auth]
command = "/usr/bin/security"
args = ["find-generic-password", "-w", "-s", "local.codex.account-switcher.switcher-provider-name"]
```

The API key is not written to `config.toml`; it is stored in macOS Keychain and read by Codex through the `security` command when the provider is active.

If a profile does not exist yet, create/login with:

```sh
/Users/cgx/Documents/Switcher/bin/codex-as --login work
```
