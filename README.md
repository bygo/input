# input

全局快捷键呼出输入框，用本地大模型批改英语语法。

## 开发

```bash
bun install
bun run tauri dev
```

默认快捷键 macOS 为 `Cmd+G`，Windows 为 `Ctrl+G`。托盘图标可打开设置或退出。

## 配置

设置写在 `~/input-workspace/settings.json`。首次启动会自动安装本地推理运行时并下载千问 2B 模型；模型文件保存在 `~/input-workspace/ollama`。

快捷键、母语等可在托盘 → 设置里改。

所有 AI 处理都在本机运行，不需要 API Key。首次下载需要网络连接，模型下载完成后可离线使用。

官网：<https://input-4gv.pages.dev>

## Windows 打包

本机在 macOS 上无法直接产出 Windows 安装包，使用 GitHub Actions：

1. 在仓库 **Actions → Windows → Run workflow** 手动触发
2. 跑完后下载 Artifact `input-windows`（NSIS `.exe`）
3. 或推送 `v*` tag，会额外打一份 Draft Release

CI 已启用 Rust 缓存；首次仍偏慢，二次及以后会明显快一些。

Windows 上截屏取词暂不可用，其余输入/翻译能力可用。
