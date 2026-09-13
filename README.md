# input

**[简体中文](#简体中文) · [English](#english)**

---

## 简体中文

`input` 是一款桌面英文写作助手。按下全局快捷键即可在当前应用中呼出输入框，用本地 AI 润色英文、翻译句子和理解表达。

### 功能

- 在其他应用中通过全局快捷键快速呼出：macOS 默认 `Cmd+G`，Windows 默认 `Ctrl+G`
- 润色英文表达、翻译文本、查看句子结构和重点短语
- 使用本地 Ollama 运行时和千问 2B 模型处理 AI 请求，无需 API Key
- 托盘菜单提供设置和退出入口

首次启动需要联网下载运行时和模型；下载完成后可以离线使用。数据保存在本机，模型文件默认位于 `~/input-workspace/ollama`，设置文件位于 `~/input-workspace/settings.json`。快捷键和母语等选项可在托盘菜单的设置中调整。

### 下载

前往[官网](https://input-4gv.pages.dev)或 [GitHub Releases](https://github.com/bygo/input/releases/latest) 下载应用。

### 开发

需要安装 [Bun](https://bun.sh/) 和 Rust 工具链。

```bash
bun install
bun run tauri dev
```

### Windows 打包

在 macOS 上无法直接生成 Windows 安装包，可使用 GitHub Actions：

1. 在仓库中打开 **Actions → Windows → Run workflow** 手动触发
2. 完成后下载 `input-windows` Artifact（NSIS `.exe`）
3. 推送 `v*` tag 也会触发构建，并创建 Draft Release

CI 已启用 Rust 缓存。Windows 目前暂不支持截屏取词，其他输入和翻译功能可用。

---

## English

`input` is a desktop writing assistant for English. Press a global shortcut to open it over your current app and use on-device AI to polish English, translate text, and understand expressions.

### Features

- Open the input panel from any app with a global shortcut: `Cmd+G` by default on macOS and `Ctrl+G` on Windows
- Polish English, translate text, and review sentence structure and key phrases
- Run AI requests locally with Ollama and the Qwen 2B model; no API key is required
- Open settings or quit from the system tray menu

The first launch requires an internet connection to download the runtime and model. After setup, the app can work offline. Data stays on your device. The model is stored in `~/input-workspace/ollama` by default, and settings are stored in `~/input-workspace/settings.json`. Change the shortcut, native language, and other options from the tray menu.

### Download

Get the app from the [website](https://input-4gv.pages.dev) or [GitHub Releases](https://github.com/bygo/input/releases/latest).

### Development

Install [Bun](https://bun.sh/) and the Rust toolchain, then run:

```bash
bun install
bun run tauri dev
```

### Windows builds

Windows installers can be built with GitHub Actions:

1. In the repository, open **Actions → Windows → Run workflow** to start a build manually
2. Download the `input-windows` artifact (an NSIS `.exe`) when the workflow completes
3. Pushing a `v*` tag also starts a build and creates a draft release

Rust build caching is enabled in CI. Screen capture and text recognition are not yet available on Windows; other input and translation features work.
