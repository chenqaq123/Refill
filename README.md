# Refill

**Run many official Codex accounts as one — switch instantly, never lose your conversation history.**
**多个官方 Codex 账号当一个用 —— 一键切换，对话历史永不丢失。**

Refill is a macOS app built around two things that normally break when you use
more than one Codex (ChatGPT) account:

1. **Multiple official accounts, first‑class.** Keep all your Codex logins side
   by side and switch between them in one click — no re‑login, no juggling
   `~/.codex`.
2. **One conversation history across all of them.** Your sessions, threads and
   projects are shared, so switching accounts never makes your history
   disappear — it's always there under whichever account is active.

On top of that it can also drive third‑party API providers (DeepSeek, OpenRouter,
…) and show your plan usage and API spend.

> Status: actively developed. Built with Tauri (Rust) + React.

![Refill — accounts](docs/screenshot-accounts.png)

### Core advantages / 核心优势
- 🔑 **Multi‑account, zero friction** — many official Codex accounts, one‑click
  switching. 多个官方账号，一键无缝切换。
- 🧵 **Unified, persistent history** — one shared conversation history that
  survives every switch. 跨账号共享、切换不丢的统一对话历史。

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
**The headline features:**
- **Multiple official Codex accounts** — keep all your ChatGPT/Codex logins and
  switch with one click. No re‑login, no manual `~/.codex` swapping; the active
  account is always clear at a glance.
- **History that survives every switch** — sessions, thread state and projects
  are shared across all accounts via a single store, so your conversation
  history is never lost or fragmented when you change accounts.

**Plus:**
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
Refill 围绕「用多个 Codex 账号时最容易坏的两件事」来设计：

1. **多个官方账号，一等公民。** 把你所有 Codex 登录并排放着，一键切换——不用重新
   登录、不用手动倒腾 `~/.codex`。
2. **跨账号共享同一条对话历史。** 会话、线程、项目都共享，切换账号历史绝不消失，
   始终在当前账号下完整可见。

在此之上，它还能驱动第三方 API provider（DeepSeek、OpenRouter…），并展示套餐额度
与 API 花费。

### 功能
**两个核心卖点：**
- **多官方 Codex 账号** —— 所有 ChatGPT/Codex 登录并存，一键切换；无需重新登录、
  无需手动切 `~/.codex`，当前账号一目了然。
- **切换不丢的历史** —— 会话、线程状态、项目列表通过统一存储在所有账号间共享，
  换账号时对话历史既不丢失也不割裂。

**此外：**
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
