# CommandPalette 升级方案

> 2026-06-10 会话产出 | 对标 Claude Code CLI + VS Code Command Palette

---

## 设计哲学

cc-gui 的命令面板 = **Claude Code 的 `Ctrl+Shift+P` 等效**——一站式访问所有 Claude 能力 + IDE 操作。

借鉴来源：Claude Code CLI 原生 slash 命令体系 + VS Code Command Palette 交互范式 + 36 个官方插件功能分类。

---

## 命令分类体系（6 大类）

| 分组 | 对标 Claude Code | 说明 |
|------|-------------------|------|
| 💬 会话 | `--resume`/`--continue`/`/clear` | 会话生命周期管理 |
| 🖥 视图 | IDE 面板 | 侧边栏/文件面板/全屏 |
| 🛡 权限与模式 | `--permission-mode` 6 种模式 | 权限控制快速切换 |
| 🧠 思考深度 | `--effort` | low/medium/high/xhigh/max/ultracode |
| 📊 上下文 | `/compact`/`/context`/`/cost` | 上下文窗口管理 |
| 🔌 工具 | 36 个官方插件 | 代码审查/精炼/安全/附件/终端 |

---

## 完整命令表（40+ 条）

### 💬 会话
- 新建会话 `Ctrl+N`
- 继续最近会话 `-c`
- 恢复会话… `--resume`
- 重命名会话 `F2`
- 删除会话 `Del`
- 清空对话 `/clear`
- 导出会话 Markdown
- 切换会话 `Ctrl+P`

### 🖥 视图
- 切换侧边栏 `Ctrl+B`
- 切换文件面板 `Ctrl+E`
- 聚焦输入框 `Ctrl+L`
- 切换全屏 `F11`
- 禅模式（隐藏全部面板）

### 🛡 权限与模式
- 默认模式 `default`
- 计划模式 `plan`
- 自动编辑 `acceptEdits`
- 自动模式 `auto`
- 完全放行 `bypassPermissions`
- 无询问模式 `dontAsk`

### 🧠 思考深度
- 低思考 / 中等 / 高 / 极高 / 最大 / 极高级(ultracode)

### 📊 上下文
- 压缩上下文 `/compact`
- 查看用量 `/context`
- 查看费用 `/cost`
- 上下文统计详情

### 🔌 工具
- 附加文件
- 代码审查 `code-review`
- 代码精炼 `code-simplifier`
- 安全检查 `security-guidance`
- 在终端中打开
- 在文件管理器打开
- 开发者工具
- 诊断 `/doctor`
- 初始化 CLAUDE.md `/init`
- 键盘快捷键参考
- 管理批准场景
- API 连接测试

### ⚙ 设置
- 打开设置 `Ctrl+,`
- 深色/浅色/跟随系统主题
- 检查更新 / 关于 cc-gui

---

## 交互升级

### 搜索匹配策略（优先级从高到低）
1. 精确匹配 id
2. 中文全拼匹配
3. 拼音首字母匹配
4. 英文 label 匹配
5. 分组名匹配
6. 描述匹配
7. 模糊容错（编辑距离 ≤2）

### 面板布局
```
┌──────────────────────────────────────────┐
│  🔍 输入命令…                     esc    │
├──────────────────────────────────────────┤
│  ⭐ 收藏                                 │  ← 手动固定
├──────────────────────────────────────────┤
│  🕐 最近使用                             │  ← 自动追踪 5 条
├──────────────────────────────────────────┤
│  💬 会话                                 │  ← 分组标题
│  ├─ 🆕 新建会话                  Ctrl+N  │
│  ├─ 📤 导出会话                         │
│  └─ 🗑 清空对话                         │
│  🖥 视图                                 │
│  ...                                     │
└──────────────────────────────────────────┘
```

### 条件显示
- 重命名/删除/清空/导出 → 有活跃会话 + 有消息
- 继续/恢复会话 → 有历史会话
- 开发者工具 → 开发模式

---

## 可扩展注册 API（P2）

```typescript
// composables/useCommandRegistry.ts
function register(cmd: Command) → 返回注销函数
function getCommands(): Command[]
```

组件按需注册，不硬编码。ChatPanel 注册自己的命令，FilePanel 注册自己的。

---

## 实施路线图

| 阶段 | 内容 | 涉及文件 | 状态 |
|------|------|----------|------|
| **P0** | i18n 命令段中英文 | `zh.json`, `en.json` | ✅ |
| **P0** | 数据结构重构：分组 + Command 接口 | `CommandPalette.vue` | ✅ |
| **P0** | 视觉升级：分组标题 + 图标 + CLI 标注 | `CommandPalette.vue` | ✅ |
| **P1** | 搜索增强：拼音首字母匹配 | `CommandPalette.vue`, `lib/pinyin.ts` | ✅ |
| **P1** | 新增 40+ 命令（对标 Claude Code） | `CommandPalette.vue` | ✅ |
| **P1** | 上下文用量弹窗 | `ContextUsageModal.vue` | ✅ |
| **P1** | 弹窗统一 (ModalShell) | `ModalShell.vue` | ✅ |
| **P2** | 最近使用 | `CommandPalette.vue` + localStorage | ✅ |
| **P2** | 可扩展注册系统 | `useCommandRegistry.ts`(新) | ✅ |
| **P2** | 组件迁移到注册方式 | `ChatPanel.vue` 等 | ✅ |
| **P2** | 管理面板（ManagePanel） | `ManagePanel.vue`(新) | ✅ |
| **P3** | 拼音库 + 命令面板测试 | `pinyin.test.ts`, `CommandPalette.test.ts`(新) | ✅ |
| **P3** | 上下文用量弹窗 | `ContextUsageModal.vue`(新) | ✅ |
| **P3** | 弹窗统一 (ModalShell) | `ModalShell.vue`(新) | ✅ |
| **P3** | 聊天命令总线 | `useChatCommandBus` | ✅ |
| **P3** | 滚动优化 + 亮色修复 | `ChatPanel.vue`, `main.css` | ✅ |

### 实施原则
1. **i18n first**：P0 一次性覆盖全部命令 key，后续直接引用
2. **渐进式**：先硬编码（P0-P1），稳定后抽象注册系统（P2）
3. **搜索优先**：快速触达比命令数量更重要
4. **对标但不照搬**：Claude Code slash 命令是交互式提示词，cc-gui 命令面板是操作入口——互补
