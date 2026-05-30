# Refill

[English](README.md) · **中文**

**多个官方 Codex 账号当一个用 —— 一键切换，对话历史永不丢失。**

Refill 是一款 macOS 应用，围绕「用多个 Codex（ChatGPT）账号时最容易坏的两件事」来设计：

1. **多个官方账号，一等公民。** 把你所有 Codex 登录并排放着，一键切换——不用重新
   登录、不用手动倒腾 `~/.codex`。
2. **跨账号共享同一条对话历史。** 会话、线程、项目都共享，切换账号历史绝不消失，
   始终在当前账号下完整可见。

在此之上，它还能驱动第三方 API provider（DeepSeek、OpenRouter…），并展示套餐额度
与 API 花费。

> 状态：持续开发中。基于 Tauri（Rust）+ React 构建。

![Refill — 账号](docs/screenshot-accounts.png)

### 核心优势
- 🔑 **多账号、零摩擦** —— 多个官方 Codex 账号，一键切换。
- 🧵 **统一、不丢的历史** —— 跨账号共享、切换不丢的同一条对话历史。

---

### 为什么做它
- 你有多个 Codex/ChatGPT 账号，用来绕开每周额度限制。
- 你也想在 Codex 里用更便宜 / 不同的模型（DeepSeek、OpenRouter、Kimi、本地
  Ollama 等）。
- 但平时切换账号会丢历史、还要重新登录。

Refill 让切换变成一键完成，并且**无论用哪个账号或 provider，都共享同一条历史**。

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

### 系统要求
- **macOS** —— 发布的 `.dmg` 是 universal 通用版（Apple Silicon **与** Intel 都支持）。
- **已安装 Codex Desktop** —— Refill 是给 Codex 切账号的，请先装好 Codex 应用。

### 安装
1. 从 [Releases](../../releases) 下载最新的 `.dmg`，把 **Refill** 拖进「应用程序」。
2. 应用尚未做 Apple 公证，首次打开会被 Gatekeeper 拦截（「无法验证开发者…」）。
   只需**放行一次**，二选一：
   - **终端：** `xattr -dr com.apple.quarantine /Applications/Refill.app`，或
   - **系统设置 → 隐私与安全性：** 先尝试打开一次，再点 **仍要打开**。
3. 使用 Chat Completions 类 provider 期间请保持 Refill 运行（本地翻译代理在应用内）。

> 想要「双击即装、零提示」的体验？那需要 Apple Developer ID 签名 + 公证——流水线
> 已在 `RELEASING.md` 备好，只差一个 Apple 开发者账号。

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

## License
MIT
