# Git 适配方案

> 状态：技术调研 | 2026-07-07

## 调研参考

| 项目 | 关键模式 |
|------|---------|
| [OpenCode Desktop](https://github.com/anomalyco/opencode) | 右侧边栏 Tab 切换（Files / Git），Electron + SolidJS |
| [OpenCode issue #15886](https://github.com/anomalyco/opencode/issues/15886) | Git Status Panel 需求——改动的文件列表、stage/unstage、commit 输入框 |
| [OpenCode issue #6521](https://github.com/anomalyco/opencode/issues/6521) | LayoutProvider + ViewRegistry 插件式面板架构 |
| [Tessera](https://github.com/horang-labs/tessera) | 多面板工作区 + Git worktree + diff 面板 |

## 目标

右侧面板改为 Tab 切换（文件 / Git），分两阶段实施。

### 阶段一：Git 状态面板（基础）

```
┌──────────────────────────────┐
│ [文件] [Git*]                │  ← Tab 切换
├──────────────────────────────┤
│ ▲ Staged (3)                 │
│   ✓ src/App.vue              │
│   ✓ src/main.ts              │    ← 点击查看 diff
├──────────────────────────────┤
│ ● Modified (5)               │
│   M src/ChatPanel.vue        │
│   M src/FileTree.vue         │
├──────────────────────────────┤
│ ＋ Untracked (2)             │
│   ＋ new-component.vue       │
├──────────────────────────────┤
│ [Commit message...]  [提交]  │
│ ☐ Amend  ☐ Push after       │
└──────────────────────────────┘
```

**Rust 后端新增命令：**
- `git_status(repo_path)` → `{ staged, modified, untracked }`
- `git_diff(repo_path, file)` → diff 文本
- `git_stage(repo_path, files)` / `git_unstage(repo_path, files)`
- `git_commit(repo_path, message, amend)`
- `git_push(repo_path)`

### 阶段二：Checkpoint + 分叉

- **手动 checkpoint**：用户点击保存当前文件快照（底层 `git commit`）
- **会话分叉**：回滚到 checkpoint + 恢复对话上下文
- **CC 自动 checkpoint**：CC 修改文件前自动 `git add -A && git commit`

## 技术方案

### 前端

| 组件 | 说明 |
|------|------|
| `GitPanel.vue` | 新组件，Git 状态列表 + 提交表单 |
| `SidePanel.vue` | 改造现有 FilePanel 外层，加 Tab 栏 |
| `GitDiffPreview.vue` | 复用现有 DiffViewer，展示选中文件的 diff |

### 后端

所有 Git 操作通过 `git2` (libgit2 的 Rust 绑定) 实现，不依赖系统 Git：
- `Cargo.toml` 加 `git2 = "0.19"`
- 新建 `src-tauri/src/git.rs`

## 与现有功能的关系

| 现有功能 | 调整 |
|------|------|
| FilePanel（文件树+预览） | 收进 `SidePanel` 的"文件" Tab |
| FilePreviewPanel（第四列编辑器） | 不变，独立存在 |
| 会话分叉 | 在 Checkpoint 基础上实现回滚 |

## 风险

- `git2` 对 Windows 的支持需要验证（CRLF 处理）
- 大型仓库的 `git_status` 性能（需异步 + 缓存）
- Checkpoint 频率过高导致仓库膨胀（手动触发为主，CC 自动为辅）
