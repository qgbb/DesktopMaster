# 桌面整理大师 (Desktop Clear Master)

> 一键整理 Windows 桌面图标，还你整洁桌面（娱乐项目，仅供学习使用）

## 简介

桌面整理大师是一款轻量级 Windows 桌面工具，通过 Win32 API 直接操控桌面图标控件，实现一键将桌面上散落的图标快速收纳到程序窗口下方区域，达到"清理桌面"的视觉效果。

程序关闭时会自动恢复桌面图标的自动排列功能，不修改任何系统文件，安全可靠。

## 功能特性

- **一键整理** —— 点击按钮即可将所有桌面图标快速收纳
- **3 秒倒计时动画** —— 带进度条的交互动画，体验流畅
- **拖拽跟随** —— 拖动程序窗口后，图标自动重新吸附到新位置
- **退出恢复** —— 关闭程序时自动恢复桌面自动排列，无残留影响
- **暗色主题** —— 简洁专业的深色 UI 界面
- **日志记录** —— 关键操作自动记录到日志文件，便于排查问题

## 技术原理

程序通过 Win32 API 查找 Windows 桌面的 `SysListView32` 控件（桌面图标所在的系统列表视图），利用 `SendMessageW` 发送 `LVM_SETITEMPOSITION` 消息来逐个重设每个图标的位置。具体流程：

1. **查找桌面图标控件**：沿 `Progman → SHELLDLL_DefView → SysListView32` 或 `WorkerW → SHELLDLL_DefView → SysListView32` 路径定位图标容器
2. **禁用自动排列**：清除 ListView 的 `LVS_AUTOARRANGE` 样式位，防止系统自动将图标排回原位
3. **移动图标**：使用乘法哈希算法为每个图标生成伪随机偏移，将其散落到程序窗口区域内
4. **窗口移动监控**：后台线程每 150ms 检测窗口位置，静止 500ms 后自动更新图标位置
5. **退出恢复**：`Drop` trait 中恢复 `LVS_AUTOARRANGE` 并发送 `LVM_ARRANGE` 重新排列

## 技术栈

| 层面 | 技术 |
|------|------|
| 语言 | Rust (edition 2021) |
| GUI 框架 | [egui](https://github.com/emilk/egui) v0.31 + [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) v0.31 |
| 系统 API | [windows-rs](https://github.com/microsoft/windows-rs) v0.58 |
| 平台 | Windows 专用 |

## 构建与运行

### 环境要求

- [Rust](https://www.rust-lang.org/tools/install) 1.70+
- Windows 10 / Windows 11

### 编译

```bash
# Debug 模式
cargo build

# Release 模式（优化体积与性能）
cargo build --release
```

### 运行

```bash
cargo run
```

编译产物位于 `target/debug/desktop-clear-master.exe`（Debug）或 `target/release/desktop-clear-master.exe`（Release）。

## 预编译下载

前往 [GitHub Releases](https://github.com/qgbb/DesktopMaster/releases) 页面下载最新版本的 `desktop-clear-master.exe`，双击即可运行。

## 使用说明

1. 启动程序后显示 **"桌面整理大师"** 主窗口
2. 点击 **"清理桌面"** 按钮
3. 等待 3 秒进度条动画
4. 桌面图标自动收纳到程序窗口下方区域
5. 关闭程序后，桌面图标恢复自动排列

> **提示**：整理后拖动程序窗口，图标会自动跟随到新位置。

## 项目结构

```
.
├── Cargo.toml          # Rust 项目配置与依赖
├── Cargo.lock          # 依赖版本锁定
├── README.md           # 项目说明
├── .gitignore
└── src/
    ├── main.rs         # GUI 应用入口：UI 布局、状态机、主题、字体
    └── desktop.rs      # Win32 核心逻辑：图标查找、移动、监控、恢复
```

## 注意事项

- 本程序**仅支持 Windows 系统**（通过 `#[cfg(windows)]` 条件编译）
- 日志文件位于 `D:/desktop_cleaner.log`，可用于调试
- 程序通过窗口标题 **"桌面整理大师"** 查找自身窗口，请勿修改标题
- 桌面图标较多时（如 > 100 个），初次整理可能需要更长时间
- 操作涉及跨进程 `SendMessage`，已在独立线程中执行，不会阻塞 UI

## License

MIT
