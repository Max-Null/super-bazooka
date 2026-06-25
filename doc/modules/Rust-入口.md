# Rust-入口

> Rust 程序入口 — 仅调用 `cc_gui::run()`，Windows 下关闭控制台窗口。

## 功能说明

- 程序入口 `main()`：调用 `cc_gui::run()` 启动 Tauri 应用
- Windows release 模式下通过 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` 隐藏控制台窗口

## 公开 API

| 类型 | 名称 | 说明 |
|------|------|------|
| function | main | 程序入口，调用 `cc_gui::run()` |

## 配置属性

本模块无对外配置属性。

## 代码示例

```rust
// main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cc_gui::run()
}
```

## 依赖说明

### 内部依赖

| 模块 | 说明 |
|------|------|
| `Rust-Tauri命令` | `cc_gui::run()` 定义在 lib.rs |

### 外部依赖（Cargo）

无直接外部依赖（由 lib.rs 间接引入）。

<!-- @generated v0.5.1 -->
<!-- @baseline commit=f67115370991f3521ab8aece00f990d651886eac generated=2026-06-26T12:00:00+08:00 -->
