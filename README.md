# input

全局快捷键呼出输入框，用大模型批改英语语法。

## 开发

```bash
bun install
bun run tauri dev
```

默认快捷键 `Cmd+G`。托盘图标可打开设置或退出。

## 配置

设置写在 `~/input-workspace/settings.json`。

设置里用图标选择服务，再选模型：

- **Claude**：Anthropic Key（`sk-ant-…`）
- **GPT**：OpenAI Key（`sk-…`）
- **OpenRouter**：OpenRouter Key（`sk-or-…`），可走多家模型
- **Grok**：[console.x.ai](https://console.x.ai) 的 xAI Key（默认）
- **Ollama**：本地，先 `ollama serve` 再 `ollama pull`，Key 可空
