# Ponytail 运行时集成方案

## 文档信息

| 项目 | 内容 |
|------|------|
| 文档标题 | Ponytail 运行时集成方案 |
| 适用项目 | `cc-gui` |
| 文档类型 | 运行时设计方案 |
| 目标版本 | Phase 1 |
| 编写日期 | 2026-06-20 |
| 编写者 | AI 助手（GPT-5.4） |
| 文档状态 | 草案 |

## 1. 方案目标

本方案用于回答一个比 UI 更关键的问题：

`cc-gui` 如何把 Ponytail 模式从“界面上的一个选项”，变成“真正会影响 Claude CLI 行为的运行时配置”。

本方案聚焦以下 4 个问题：

1. Ponytail 模式状态存储在何处
2. GUI 如何读写该状态
3. Rust 启动 Claude CLI 时如何让该状态生效
4. 在桌面 GUI 环境下，如何替代 VSCode hooks 不可用的缺口

## 2. 当前项目现状

当前项目已经具备这些基础能力：

- 前端 `settings` store 同时管理：
  - `~/.claude/settings.json` 中的 Claude 配置
  - `localStorage` 中的 GUI 偏好
- Rust 后端已提供：
  - `get_claude_settings()`
  - `set_claude_settings()`
  - `spawn_claude_session()`
- CLI 启动链路已经明确接入：
  - `permission_mode`
  - `effort`
  - `ultracode`

但当前仍然缺少 Ponytail 的正式运行时契约：

- 没有 `ponytailMode` 字段
- 没有 Ponytail 配置读写接口
- 没有 Ponytail 模式注入 Claude CLI 的流程
- 没有用于替代 hooks 的桌面版 session 激活机制

## 3. 设计原则

1. Ponytail 必须成为项目内的显式运行时配置，而不是依赖用户手工编辑外部文件
2. GUI 配置和 CLI 运行时之间要有单一可信链路
3. 尽量复用当前 `settings store -> tauri-bridge -> Rust command -> settings.json / runtime env` 的现有结构
4. 桌面版不依赖 VSCode hooks；必须由 `cc-gui` 自己负责会话开始时的激活与注入
5. 优先做“可稳定生效”的最小闭环，再考虑状态展示和高级管理界面

## 4. 运行时契约设计

### 4.1 新增模式枚举

建议新增 `PonytailMode`：

```ts
export type PonytailMode = "off" | "lite" | "full" | "ultra";
```

说明：

- `off`：关闭 Ponytail
- `lite`：只给出更懒替代方案，不强制执行
- `full`：默认模式，启用完整懒人阶梯
- `ultra`：极端 YAGNI / 先删后加

### 4.2 新增配置字段

建议在前端 `settings store` 中新增：

```ts
const ponytailMode = ref<PonytailMode>("off");
```

建议在 Rust `ClaudeSettings` 中新增：

```rust
ponytail_mode: String,
```

### 4.3 存储层拆分

建议将 Ponytail 配置拆成两层：

#### 层 1：GUI 偏好层

位置：

- 浏览器 `localStorage`

作用：

- 保存当前 GUI 选中的 Ponytail 模式
- 让界面状态在下次打开时保持一致

#### 层 2：Claude 运行时配置层

位置建议：

- `~/.claude/settings.json`

字段建议：

```json
{
  "env": {
    "PONYTAIL_DEFAULT_MODE": "full"
  }
}
```

说明：

- 该字段不要求 Claude CLI 原生认识
- 其作用是为 Ponytail 插件 / 规则系统提供一个稳定的运行时来源
- 也方便 GUI、CLI、外部脚本共享一个配置位置

### 4.4 为什么不用只写 localStorage

只写 `localStorage` 的问题是：

- Rust 启动子进程时读不到
- CLI 外部运行也无法复用该状态
- 无法和现有 `settings.json` 同步管理

因此，`localStorage` 只能保存 GUI 偏好，不能作为真正运行时来源。

## 5. 配置流设计

### 5.1 前端读取流程

启动时：

1. `settings store` 从 `localStorage` 读取 GUI 偏好
2. 调用 `get_claude_settings()`
3. 从 `settings.json.env.PONYTAIL_DEFAULT_MODE` 读取运行时值
4. 若 GUI 偏好未显式覆盖，则使用运行时值初始化 `ponytailMode`

### 5.2 前端写入流程

当用户在 GUI 中切换 Ponytail 模式时：

1. 更新 `settings store.ponytailMode`
2. 写入 `localStorage`
3. 调用 `set_claude_settings(...)`
4. Rust 将 `PONYTAIL_DEFAULT_MODE` 写回 `~/.claude/settings.json`

### 5.3 Rust 配置接口扩展

当前接口：

```ts
setClaudeSettings(apiKey, baseUrl, model, effort, permissionMode)
```

建议改为：

```ts
setClaudeSettings(apiKey, baseUrl, model, effort, permissionMode, ponytailMode)
```

Rust 侧同步扩展：

```rust
fn set_claude_settings(
    api_key: String,
    base_url: String,
    model: String,
    effort: String,
    permission_mode: String,
    ponytail_mode: String,
) -> Result<(), String>
```

`get_claude_settings()` 同样返回 `ponytail_mode`。

## 6. CLI 启动时的生效方案

这是整个方案最关键的一节。

### 6.1 目标

确保 Ponytail 模式不只是“保存在配置文件里”，而是每次由 `cc-gui` 启动 Claude CLI 时都明确生效。

### 6.2 推荐方案：双通道注入

建议采用“双通道注入”：

#### 通道 A：持久化环境变量

在 `~/.claude/settings.json` 中写：

```json
{
  "env": {
    "PONYTAIL_DEFAULT_MODE": "full"
  }
}
```

作用：

- 作为稳定配置来源
- 让 GUI 以外的 Claude CLI 运行也能继承默认模式

#### 通道 B：会话启动时显式注入

在 `spawn_claude_session()` 启动子进程时，为该次进程额外注入环境变量：

```rust
command.env("PONYTAIL_DEFAULT_MODE", ponytail_mode);
```

作用：

- 保证本次会话一定使用 GUI 当前选择的值
- 即便 `settings.json` 尚未刷新或被外部修改，也不影响当前会话

### 6.3 为什么要双通道

只写配置文件的问题：

- 当前会话不一定即时读取到最新值
- 受外部手工编辑影响

只写进程环境变量的问题：

- 下次启动 GUI 或单独运行 CLI 时无法保留状态

双通道的好处是：

- 配置持久
- 会话确定
- 便于排查

## 7. 桌面版替代 hooks 方案

Ponytail 在原设计中依赖：

- `SessionStart`
- `UserPromptSubmit`

但这类 hooks 在 VSCode 扩展环境下存在已知问题。对于 `cc-gui`，更合理的做法是不依赖这些 hooks，而是由 GUI 主动承担它们的职责。

### 7.1 替代 SessionStart

在 `create_session()` 或第一次 `send_message()` 之前，由 `cc-gui` 主动执行“模式激活”。

建议动作：

1. 确定当前 `ponytailMode`
2. 将该模式写入运行时环境
3. 在会话元数据中记录本次会话启用的 Ponytail 模式
4. 可选：在前端消息流里追加一条 system note，提示本轮模式

### 7.2 替代 UserPromptSubmit

当用户发送消息时，GUI 不需要依赖外部 hook 去解析 `/ponytail full` 这类命令，而是可以自行接管：

1. 在发送前检查输入是否匹配：
   - `/ponytail off`
   - `/ponytail lite`
   - `/ponytail full`
   - `/ponytail ultra`
2. 若匹配，则直接更新 `settings store.ponytailMode`
3. 同步写回配置
4. 在当前会话中追加一条轻量提示
5. 不把该控制命令继续发送给 Claude CLI，避免重复解释

这意味着 Ponytail 模式切换在桌面 GUI 中是“产品内建行为”，而不是依赖 CLI 插件 hook 的旁路行为。

### 7.3 会话级与全局级策略

建议 Phase 1 先采用：

- `全局默认 + 当前会话即时生效`

具体规则：

- GUI 中切换 Ponytail 模式后，更新全局默认值
- 后续新会话默认采用新值
- 正在运行的新一轮消息发送也采用新值

不建议 Phase 1 一开始就引入：

- 项目级覆盖
- 会话级覆盖持久化
- workspace 特化策略

## 8. 前后端改动点

### 8.1 前端

建议改动：

- `src/stores/settings.ts`
  - 新增 `PonytailMode`
  - 新增 `ponytailMode`
  - 扩展 `getClaudeSettings / setClaudeSettings` 调用链
- `src/lib/tauri-bridge.ts`
  - 扩展 `getClaudeSettings()` 返回值
  - 扩展 `setClaudeSettings()` 参数
  - 可选新增 `setPonytailMode()` 的单独封装
- `src/components/settings/SettingsPanel.vue`
  - 增加 Ponytail 模式选择 UI
- `src/components/shared/CommandPalette.vue`
  - 增加 `/ponytail-*` 相关命令
- `src/components/chat/ChatPanel.vue`
  - 拦截 `/ponytail mode` 控制命令

### 8.2 Rust

建议改动：

- `src-tauri/src/lib.rs`
  - 扩展 `ClaudeSettings`
  - 扩展 `get_claude_settings()`
  - 扩展 `set_claude_settings()`
- `src-tauri/src/process.rs`
  - 扩展 `SpawnParams`
  - 在 `spawn_claude_session()` 中注入 `PONYTAIL_DEFAULT_MODE`
- `src-tauri/src/session.rs`
  - 可选记录每个会话使用的 Ponytail 模式

## 9. Phase 1 范围

Phase 1 只做最小闭环，不做完整产品化。

### 包含内容

- 定义 `PonytailMode`
- GUI 可读写 Ponytail 模式
- `~/.claude/settings.json` 可保存 `PONYTAIL_DEFAULT_MODE`
- Rust 启动 CLI 时为子进程显式注入该变量
- 命令面板可触发 Ponytail 模式切换

### 不包含内容

- `ManagePanel` Ponytail 专属 Tab
- `ponytail:` 技术债扫描
- 增益记分牌
- 每工作区 / 每项目差异化配置
- 完整会话内可视化状态历史

## 10. 验收标准

完成 Phase 1 后，需要满足以下条件：

1. 在 GUI 中切换 Ponytail 模式后，刷新应用仍能保留当前值
2. `~/.claude/settings.json` 中可看到 `PONYTAIL_DEFAULT_MODE`
3. Rust 启动的 Claude CLI 子进程可以拿到对应环境变量
4. 使用 `/ponytail full`、`/ponytail off` 等命令可在 GUI 内完成切换
5. 新会话与新消息发送使用最新 Ponytail 模式

## 11. 风险评估

### 11.1 插件行为与环境变量契约风险

- 风险：Ponytail 最终是否以 `PONYTAIL_DEFAULT_MODE` 为唯一可信来源，需要与插件自身实现保持一致
- 缓解：
  - 在真正落地前确认 Ponytail 当前版本读取优先级
  - 若插件更依赖 `config.json`，则补一个配置文件写入适配层

### 11.2 配置源冲突风险

- 风险：`localStorage`、`settings.json`、运行中会话环境变量三者可能短时间不同步
- 缓解：
  - 明确优先级为：当前进程环境 > settings.json > localStorage 恢复值
  - 当前会话始终以 Rust 注入值为准

### 11.3 控制命令拦截风险

- 风险：若前端拦截 `/ponytail full`，需要确保不会误拦普通文本
- 缓解：
  - 仅对精确匹配的控制命令做拦截
  - 其他以 `/ponytail` 开头但不符合模式切换格式的输入，继续透传给 CLI

## 12. 推荐实施顺序

建议按以下顺序推进：

1. 扩展 `settings` 数据模型与 Tauri 接口
2. 在 Rust 中补 `PONYTAIL_DEFAULT_MODE` 读写
3. 在 `spawn_claude_session()` 中注入环境变量
4. 在 `SettingsPanel` 中增加 Ponytail 模式 UI
5. 在 `CommandPalette` 和发送链路中支持 `/ponytail mode`
6. 最后再加状态展示

## 13. 结论

Ponytail 是否值得集成，首先不是一个 UI 问题，而是一个运行时契约问题。

本方案给出的核心结论是：

- Ponytail 模式必须成为 `cc-gui` 的正式运行时配置
- 配置需要同时落到持久层和当前进程环境
- 桌面 GUI 不应依赖外部 hooks，而应自行接管 session 激活和模式切换

只要这个闭环打通，后续不论是状态标签、管理面板还是技术债视图，都会有稳定的底座可以继续扩展。
