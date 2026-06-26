# Tauri Icons

Tauri 需要以下格式的图标文件：

- `128x128.png`, `256x256.png`, `512x512.png` (Linux)
- `icon.icns` (macOS)
- `icon.ico` (Windows)

## 从 SVG 生成图标

可以使用 `tauri icon` 命令从现有 favicon.svg 一键生成：

```bash
pnpm add -D @tauri-apps/cli
pnpm tauri icon ../../web/public/favicon.svg
```

运行上述命令后，所有需要的图标文件会自动生成到此目录。
