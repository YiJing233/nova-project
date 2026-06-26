# Nova 桌面应用设计文档（Tauri 集成方案）

> **日期**：2026-06-25  
> **方案**：Tauri + Go Sidecar（最小侵入，保留现有命令行模式）

---

## 1. 概述

### 1.1 目标
- 将 Nova 打包为开箱即用的桌面应用，支持 Mac/Linux/Windows 三个平台
- **保留完整的命令行模式**，与桌面模式共存，不破坏现有用户习惯
- 实现的特性：原生窗口、文件关联、内置自动更新、标准菜单

### 1.2 架构选型
采用 **Tauri Sidecar 模式**：
- **Tauri**：负责桌面壳、窗口、文件关联、自动更新
- **Go Sidecar**：原有的 nova 二进制作为 HTTP 服务，业务逻辑保持不变
- **通信**：Tauri 启动 Go 进程 → 监听 Go 的 stdout 输出获取端口 → 等待 HTTP 健康检查 → 加载本地 Web UI

---

## 2. 目录结构

新增文件如下，现有代码几乎不需要修改：
```
/workspace
├── desktop/                          # 新增：桌面应用目录
│   └── tauri/
│       ├── Cargo.toml                # Tauri Rust 依赖
│       ├── tauri.conf.json           # Tauri 配置：窗口、文件关联、updater、打包
│       ├── icons/                    # 多平台图标
│       │   ├── icon.icns             # macOS
│       │   ├── icon.ico              # Windows
│       │   └── 128x128.png 等        # Linux 图标集
│       └── src/
│           └── main.rs               # 桌面壳主入口：sidecar 生命周期、启动逻辑
├── web/                              # 现有：前端完全不变
├── cmd/nova/                         # 现有：Go 入口需加 --desktop 标志
├── package.json                      # 根目录：新增 tauri scripts
├── scripts/
│   └── build-tauri.sh                # 新增：Tauri 跨平台打包脚本
└── .github/workflows/
    └── release.yml                   # 现有：需新增 Tauri 构建步骤
```

---

## 3. Go 后端最小改动

仅需对 [cmd/nova/main.go](file:///workspace/cmd/nova/main.go) 做以下修改：

### 3.1 新增 --desktop 标志
- 在现有 flags 基础上，新增 `--desktop` 布尔标志
- 桌面模式行为：
  - 绑定 `127.0.0.1` 而非 `0.0.0.0`（防止意外暴露）
  - **不自动打开浏览器**（交给 Tauri 原生窗口）
  - **启动后立即输出**：`[nova-desktop-ready] port=xxxx url=http://127.0.0.1:xxxx`，供 Tauri 解析
  - 若父进程退出（Tauri 关闭），自动退出自身

---

## 4. Tauri 层实现要点

### 4.1 桌面启动流程（main.rs）
1. **启动 Sidecar**：使用 Tauri 内置的 `Command::new_sidecar` 启动 Go binary
2. **解析端口**：读取 sidecar 的 stdout，匹配 `[nova-desktop-ready] port=(\d+)` 正则
3. **健康检查**：轮询 `http://127.0.0.1:<port>`，直到返回 200
4. **创建窗口**：加载本地 URL，应用窗口配置（记住大小、全屏支持等）
5. **文件关联支持**：如果启动时收到文件/文件夹参数，将其通过 `--workspace` 传递给 sidecar

### 4.2 tauri.conf.json 关键配置
- **windows**：窗口配置（标题、尺寸、最小尺寸、装饰器、透明等）
- **bundle**：打包配置（图标、资源文件、sidecar 二进制、targets）
- **updater**：自动更新配置（使用 GitHub Releases 作为源）
- **fileAssociations**：文件关联（绑定 `.nova` 工作区、`.toml` 配置）
- **security**：CSP 配置（允许 `localhost:*` 资源访问）

### 4.3 图标制作
使用现有 `web/public/favicon.svg` 转换为 Tauri 需要的多平台图标集：
- macOS：icns
- Windows：ico
- Linux：png 多尺寸（128x128, 256x256, 512x512）

---

## 5. CI/CD 调整

### 5.1 release.yml 新增步骤
现有 [build-github-release.sh](file:///workspace/scripts/build-github-release.sh) 保持不变（继续生成 tar.gz/zip 命令行包）。

新增 Tauri 构建：
- 使用 Tauri 官方 `tauri-action`，在 macOS/Windows/Linux 三个 runner 上分别构建
- macOS：构建 `.app` + `.dmg`（universal binary 合并 arm64 + x64）
- Windows：构建 `.exe` + NSIS 安装包
- Linux：构建 `.AppImage` + `.deb`
- 所有桌面包上传到同一个 GitHub Release，与命令行包并列

---

## 6. 开发体验

- **桌面开发模式**：`pnpm tauri dev` → 自动启动 Go 后端 + Vite 热重载 + Tauri 窗口
- **命令行开发**：保持 `./bootstrap.sh` 不变
- **根目录 package.json**：新增 `"scripts": { "tauri": "tauri" }` 等

---

## 7. 包体积预估

| 平台       | 包类型       | 预估大小 |
|------------|--------------|----------|
| macOS      | .dmg         | ~25MB    |
| Windows    | .exe/NSIS    | ~20MB    |
| Linux      | .AppImage    | ~25MB    |
| (对比：Electron 通常 ~150MB+) | |

---

## 8. 风险与注意事项

### 8.1 Sidecar 二进制路径
- Tauri 在不同平台、打包前后的资源路径有差异，需正确配置 `tauri.conf.json` 的 `bundle.resources` 和 `bundle.targets`

### 8.2 父子进程生命周期
- 确保 Tauri 窗口关闭时，Go sidecar 也能正确退出（Go 侧检测父进程退出）

### 8.3 向后兼容
- 完全保证现有命令行用户无感知，所有现有行为保持不变
