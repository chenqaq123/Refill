# Codex Account Switcher

This repo keeps multiple Codex logins separated by profile. It does not copy tokens into scripts. Each profile gets its own `CODEX_HOME` directory under `~/.codex-profiles`.

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
- save the current `~/.codex` automatically using account info, without asking for a profile name
- switch profiles with one click by quitting Codex, hydrating missing Desktop support files, relinking `~/.codex`, and reopening Codex
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

If a profile does not exist yet, create/login with:

```sh
/Users/cgx/Documents/Switcher/bin/codex-as --login work
```
