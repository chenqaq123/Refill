# Refill

**One switcher for all your Codex accounts and API providers — with one shared history.**
**一个开关，管理你所有的 Codex 账号与 API provider —— 共享同一条对话历史。**

Refill is a macOS app that lets you switch between multiple official Codex
(ChatGPT) accounts and third‑party API providers in one click, while keeping a
single, unified conversation history across all of them. It also shows how much
of each account's plan you've burned, and what your API spend looks like.

> Status: actively developed. Built with Tauri (Rust) + React.

---

## English

### Why
- You juggle several Codex/ChatGPT accounts to get around weekly rate limits.
- You also want to use cheaper / different models (DeepSeek, OpenRouter, Kimi,
  local Ollama…) inside Codex.
- Switching normally means losing your conversation history and re‑logging in.

Refill makes switching instant and **keeps one history** no matter which account
or provider is active.

### Features
- **One‑click account switching** — official Codex accounts and API providers.
- **Shared conversation history** — sessions, thread state and projects are
  shared across every profile, so history never disappears when you switch.
- **Use Chat‑Completions‑only providers with Codex** — Codex only speaks the
  OpenAI *Responses API*. Refill runs a built‑in local proxy that translates to
  and from Chat Completions, so providers like **DeepSeek** work transparently —
  with **streaming, reasoning (thinking) and tool calls** supported.
- **Usage & cost**
  - *Official accounts*: per‑account weekly / 5‑hour quota windows, charted over
    time, so you can see exactly how much each period lets you consume.
  - *API providers*: real token usage with editable per‑model pricing → estimated
    cost, plus a request log.
- **Frictionless setup** — provider presets (DeepSeek / OpenRouter / Kimi /
  Ollama), a live connection test, and automatic protocol detection.
- **Safe by default** — API keys live in the macOS Keychain (never in config
  files), everything stays local, no telemetry.
- **Built to grow** — the tool rail is ready for more than Codex (Claude Code,
  Gemini CLI… are scaffolded).

### Install
1. Download the latest `.dmg` from [Releases](../../releases).
2. Open it and drag **Refill** to Applications.
3. The app is not yet notarized, so on first launch right‑click it and choose
   **Open** (or run `xattr -dr com.apple.quarantine /Applications/Refill.app`).

### Build from source
Requires Rust, Node 20+, and the Tauri prerequisites.
```bash
npm install
npm run tauri:build      # produces .app + .dmg under src-tauri/target/release/bundle
# or for development:
npm run tauri:dev
```

### How it works
- Each account/provider is a profile under `~/.codex-profiles/<id>`; `~/.codex`
  is symlinked to the active one.
- Sessions, the thread‑state SQLite DBs and project list are shared via a
  `_shared-history` folder, and the recorded provider is realigned on switch so
  history stays visible under whatever account is active.
- For Chat‑Completions‑only providers, the profile's `config.toml` points Codex
  at `127.0.0.1:8765`, where Refill's proxy translates Responses ⇄ Chat.

### Privacy
API keys are stored in the macOS Keychain. Conversation history, usage records
and logs stay on your machine under `~/.codex-profiles`. Refill sends nothing
anywhere except the API requests you make to your own providers.

---

## 中文

### 为什么做它
- 你有多个 Codex/ChatGPT 账号，用来绕开每周额度限制。
- 你也想在 Codex 里用更便宜 / 不同的模型（DeepSeek、OpenRouter、Kimi、本地
  Ollama 等）。
- 但平时切换账号会丢历史、还要重新登录。

Refill 让切换变成一键完成，并且**无论用哪个账号或 provider，都共享同一条历史**。

### 功能
- **一键切换账号** —— 官方 Codex 账号与 API provider 都支持。
- **共享对话历史** —— 会话、线程状态、项目列表在所有 profile 间共享，切换账号
  历史也不会消失。
- **让只支持 Chat Completions 的 provider 也能用 Codex** —— Codex 只会说 OpenAI
  的 *Responses API*。Refill 内置一个本地代理做双向协议翻译，于是 **DeepSeek**
  这类服务可以透明接入，并且**支持流式输出、思维链、工具调用**。
- **用量与成本**
  - *官方账号*：按账号展示「周 / 5 小时」额度窗口，并用柱状图呈现每个周期的
    消耗，让你清楚每个阶段到底能用多少。
  - *API provider*：真实 token 用量 + 可编辑的每模型单价 → 预估花费，外加请求日志。
- **顺滑的添加流程** —— 内置 provider 预设（DeepSeek / OpenRouter / Kimi /
  Ollama）、实时连通性测试、自动协议探测。
- **默认安全** —— API Key 存进 macOS 钥匙串（绝不写进配置文件），数据全部本地，
  无任何遥测。
- **为扩展而生** —— 左侧工具栏已为 Codex 之外的工具（Claude Code、Gemini CLI…）
  预留位置。

### 安装
1. 从 [Releases](../../releases) 下载最新的 `.dmg`。
2. 打开后把 **Refill** 拖进「应用程序」。
3. 应用尚未做 Apple 公证，首次打开请**右键 → 打开**（或执行
   `xattr -dr com.apple.quarantine /Applications/Refill.app`）。

### 从源码构建
需要 Rust、Node 20+ 以及 Tauri 的依赖环境。
```bash
npm install
npm run tauri:build      # 产物在 src-tauri/target/release/bundle 下
# 开发模式：
npm run tauri:dev
```

### 工作原理
- 每个账号 / provider 是 `~/.codex-profiles/<id>` 下的一个 profile，`~/.codex`
  软链到当前激活的那个。
- 会话、线程状态 SQLite 库、项目列表通过 `_shared-history` 目录共享；切换时会
  把记录里的 provider 对齐，保证历史在当前账号下依然可见。
- 对于只支持 Chat Completions 的 provider，profile 的 `config.toml` 把 Codex
  指向 `127.0.0.1:8765`，由 Refill 的代理完成 Responses ⇄ Chat 的翻译。

### 隐私
API Key 存于 macOS 钥匙串。对话历史、用量记录与日志都保存在本机
`~/.codex-profiles` 下。除了你主动发往自己 provider 的 API 请求，Refill 不向任何
地方发送数据。

---

## License
MIT
