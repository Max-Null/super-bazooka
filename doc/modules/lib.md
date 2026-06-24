# lib

## 功能说明

工具库——Tauri 桥接层、通用工具函数、拼音搜索、测试 Mock。前端与 Rust 后端通信的唯一通道。

- tauri-bridge.ts：封装所有 `invoke()` 调用，统一错误处理
- utils.ts：通用工具函数
- pinyin.ts：拼音首字母搜索（3755 常用汉字）
- tauri-mock.ts：完整 Tauri API mock，供浏览器内 E2E 测试

## 公开 API

| 类型 | 名称 | 说明 |
|------|------|------|
| function | tauri-bridge | Tauri IPC 桥接层，所有 `invoke()` 调用统一入口，禁止组件直接调用 `invoke()` |
| function | utils | 通用工具函数 |
| function | pinyin | 拼音首字母搜索，覆盖 3755 个常用汉字，供 CommandPalette 使用 |
| function | tauri-mock | Tauri API Mock，Playwright E2E 测试在浏览器中运行无需 Tauri |

## 依赖说明

### 外部依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| @tauri-apps/api | - | Tauri 前端 API |
