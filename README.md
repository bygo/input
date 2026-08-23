# input

全局快捷键呼出输入框，用大模型批改英语语法。

## 开发

```bash
bun install
bun run tauri dev
```

默认快捷键 macOS 为 `Cmd+G`，Windows 为 `Ctrl+G`。托盘图标可打开设置或退出。

## 配置

设置写在 `~/input-workspace/settings.json`。

设置里用图标选择服务，再选模型：

- **Claude**：Anthropic Key（`sk-ant-…`）
- **GPT**：OpenAI Key（`sk-…`）
- **OpenRouter**：OpenRouter Key（`sk-or-…`），可走多家模型
- **Grok**：[console.x.ai](https://console.x.ai) 的 xAI Key（默认）
- **Ollama**：本地，先 `ollama serve` 再 `ollama pull`，Key 可空

## Windows 打包

本机在 macOS 上无法直接产出 Windows 安装包，使用 GitHub Actions：

1. 在仓库 **Actions → Windows → Run workflow** 手动触发
2. 跑完后下载 Artifact `input-windows`（NSIS `.exe`）
3. 或推送 `v*` tag，会额外打一份 Draft Release

CI 已启用 Rust 缓存；首次仍偏慢，二次及以后会明显快一些。

Windows 上截屏取词暂不可用，其余输入/翻译能力可用。
