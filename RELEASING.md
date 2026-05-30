# Releasing Refill (signing · notarization · auto-update)

This is the Phase 4 distribution pipeline. The code scaffolding is in place;
the steps below need **your Apple Developer account** and one-time secret setup.
Until then, local `npm run tauri:build` keeps producing an unsigned `.app`
(which is why install currently needs `xattr -dr com.apple.quarantine`).

## What's already done in the repo
- `.github/workflows/release.yml` — tag-triggered build → sign → notarize →
  GitHub Release with a Tauri updater `latest.json`.
- `src-tauri/entitlements.plist` — hardened-runtime entitlements for notarization.
- Crash logging — panics are appended to
  `~/.codex-profiles/_shared-history/crash.log`.
- An updater signing keypair was generated locally at `.tauri-keys/`
  (gitignored). **Public key** (safe to commit / put in config):

  ```
  dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDcwOUMxMURGRTZBRDhGMTEKUldRUmo2M20zeEdjY0Y0VVdPNHlOSmZFOXp2YlF6U1J0U0pqc0FYY1NvNFBNSmp2MVFFYXRRS0EK
  ```

## 1. Apple Developer ID (for signing + notarization)
1. Join the Apple Developer Program ($99/yr) and create a **Developer ID
   Application** certificate in Xcode or the developer portal.
2. Export it as a `.p12` (with a password).
3. Create an **app-specific password** at appleid.apple.com for notarization.
4. Note your 10-character **Team ID**.

## 2. Repository secrets (Settings → Secrets and variables → Actions)
| Secret | Value |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | contents of `.tauri-keys/refill-updater.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | the key password (empty string if none) |
| `APPLE_CERTIFICATE` | `base64 -i DeveloperID.p12` output |
| `APPLE_CERTIFICATE_PASSWORD` | the `.p12` password |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID` | your Apple ID email |
| `APPLE_PASSWORD` | the app-specific password |
| `APPLE_TEAM_ID` | your Team ID |

## 3. Enable code signing for the bundle
Add to `src-tauri/tauri.conf.json` under `bundle`:

```json
"macOS": {
  "entitlements": "entitlements.plist",
  "signingIdentity": "Developer ID Application: Name (TEAMID)"
}
```

(CI reads the identity from `APPLE_SIGNING_IDENTITY`; local unsigned builds
still work because no identity is present.)

## 4. Cut a release
```bash
git tag v4.0.1
git push origin v4.0.1
```
The workflow builds a universal `.app` + `.dmg`, signs & notarizes them, and
publishes a **draft** GitHub Release plus `latest.json`. Review and publish it.

## 5. Turn on in-app auto-update (one-time, after the first published release)
The in-app updater is intentionally **not enabled yet** — it can only be
verified against a real published `latest.json`, and wiring it blind risks the
ACL. To enable once a release exists:

1. `npm i @tauri-apps/plugin-updater @tauri-apps/plugin-process`
2. `src-tauri/Cargo.toml`: add `tauri-plugin-updater = "2"` and
   `tauri-plugin-process = "2"`.
3. `src-tauri/src/lib.rs`: in the builder add
   `.plugin(tauri_plugin_updater::Builder::new().build())`
   `.plugin(tauri_plugin_process::init())`.
4. `src-tauri/tauri.conf.json`: add
   ```json
   "plugins": {
     "updater": {
       "endpoints": ["https://github.com/chenqaq123/Refill/releases/latest/download/latest.json"],
       "pubkey": "<the public key above>"
     }
   }
   ```
5. Add `src-tauri/capabilities/default.json` granting `updater:default` and
   `process:default` (plus the core defaults the app already uses).
6. Add a "检查更新" button that calls `check()` → `downloadAndInstall()` →
   `relaunch()`.

After that, `git tag vX.Y.Z && git push` ships an update users get in-app.
