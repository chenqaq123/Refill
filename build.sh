#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
app="$root/dist/Codex Account Switcher.app"
contents="$app/Contents"
macos="$contents/MacOS"
resources="$contents/Resources"
module_cache="$root/.build/module-cache"

rm -rf "$app"
mkdir -p "$macos" "$resources" "$module_cache"

swift_sources=("$root"/Sources/CodexSwitcher/*.swift)

CLANG_MODULE_CACHE_PATH="$module_cache" swiftc "${swift_sources[@]}" \
  -parse-as-library \
  -framework AppKit \
  -framework SwiftUI \
  -o "$macos/CodexAccountSwitcher"

cp "$root/bin/codex-as" "$resources/codex-as"
chmod +x "$resources/codex-as"

cat > "$contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>zh_CN</string>
  <key>CFBundleExecutable</key>
  <string>CodexAccountSwitcher</string>
  <key>CFBundleIdentifier</key>
  <string>local.codex.account-switcher</string>
  <key>CFBundleName</key>
  <string>Codex Account Switcher</string>
  <key>CFBundleDisplayName</key>
  <string>Codex Account Switcher</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>2.0.0</string>
  <key>CFBundleVersion</key>
  <string>2</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHumanReadableCopyright</key>
  <string>Local utility</string>
</dict>
</plist>
PLIST

chmod +x "$macos/CodexAccountSwitcher"
echo "$app"
