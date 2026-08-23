# input

全局快捷键呼出输入框，用大模型批改英语语法。

## 开发

```bash
bun install
bun run tauri dev
```

默认快捷键 macOS 为 `Cmd+G`，Windows 为 `Ctrl+G`。托盘图标可打开设置或退出。

## 配置

设置写在 `~/input-workspace/settings.json`。默认走内置云端，安装即可用，**不必配 Key**。

快捷键、母语等可在托盘 → 设置里改。

### 自备 Key / 本地模型（兜底，不是功能）

配自己的 API Key、或安装本地模型，只是云端不可用时的退路，**不作为产品能力对外宣传**。日常用法以开箱即用为准。

## Windows 打包

本机在 macOS 上无法直接产出 Windows 安装包，使用 GitHub Actions：

1. 在仓库 **Actions → Windows → Run workflow** 手动触发
2. 跑完后下载 Artifact `input-windows`（NSIS `.exe`）
3. 或推送 `v*` tag，会额外打一份 Draft Release

CI 已启用 Rust 缓存；首次仍偏慢，二次及以后会明显快一些。

Windows 上截屏取词暂不可用，其余输入/翻译能力可用。
