# Phase 8 计划

> 2026-06-17 规划 | 2026-06-18 更新 — Settings 面板增强 + MCP 状态监控已完成

---

## 一、MCP 实时状态监控

### 目标
在管理面板 MCP Tab 中，除了显示已配置的 MCP 服务器列表，还要显示**运行时连接状态**（已连接/断开/错误）。

### 数据来源
1. **静态配置**：`~/.claude/settings.json` 或 `.mcp.json` 中的 `mcpServers` 列表
2. **运行时状态**：Claude CLI 启动时发出的 `system/init` 事件中包含 `tools` 列表，MCP 服务器的工具名带有 `mcp__<server>__<tool>` 前缀

### 实现思路
```
                  静态配置                        运行时状态
           ┌─────────────────┐            ┌─────────────────┐
           │ .mcp.json        │            │ system/init 事件  │
           │ settings.json    │            │ tools 列表        │
           │ mcpServers: {    │            │ mcp__playwright__ │
           │   playwright: {} │            │ mcp__memory__    │
           │   context7: {}   │            │ mcp__time__      │
           │ }                │            └────────┬────────┘
           └────────┬────────┘                      │
                    │                               │
                    └───────────┬───────────────────┘
                                │
                     ┌──────────▼──────────┐
                     │  状态对比 & 展示     │
                     │                     │
                     │ playwright  🟢 已连接│
                     │ context7    🟢 已连接│
                     │ memory      🟡 连接中│
                     │ time        🔴 断开  │
                     └─────────────────────┘
```

### 具体步骤
1. **Rust 端**：在 `protocol.rs` 解析 `system/init` 事件时，提取 `tools` 列表中 `mcp__<server>__*` 格式的工具名，聚合为 MCP 服务器列表，通过 Tauri event 发送到前端
2. **前端**：ManagePanel MCP Tab 读取运行时状态，与静态配置做对比
3. **状态标识**：
   - 🟢 已连接：配置存在 + 运行时工具可见
   - 🟡 连接中：配置存在 + 等待 system/init
   - 🔴 断开/错误：配置存在 + 运行时无对应工具
   - ⚪ 未配置：仅静态配置存在，无运行时信息

### 涉及文件
- `src-tauri/src/protocol.rs` — 解析 system/init 提取 MCP 工具
- `src/composables/useStreamProcessor.ts` — 新增 mcp-status 事件处理
- `src/components/shared/ManagePanel.vue` — MCP Tab 状态展示

### 估时
约 2-3 小时

### ✅ 已完成（2026-06-18）

实际实现与计划有差异：
- **数据源修正**：静态 MCP 配置实际存储在 `~/.claude.json`（非 `settings.json`，后者被 CLI 静默忽略）。`~/.claude.json` 的 `projects.<cwd>.disabledMcpServers` 支持启用/禁用。
- **运行状态**：从 `system/init` 的 `mcp_servers` 原生字段读取（非 tools 解析），更准确。
- **AI 描述生成**：用 `CLAUDE_CODE_SUBAGENT_MODEL` 模型为无描述 MCP 自动生成中文简介，缓存到 `item_descriptions` 表。
- **过滤**：只显示已启用插件的 MCP。
- **额外**：ModalShell 新增 `#footer` 插槽，设置页三区平铺布局。

---

## 二、自动更新 / 代码签名

### 目标
cc-gui 发布新版本后，用户应用内收到更新通知，一键下载安装。Windows 安装包经过代码签名，不触发 SmartScreen 警告。

### 技术方案

#### 2.1 自动更新
使用 Tauri 官方插件 `tauri-plugin-updater`：

```
发布流程：
  GitHub Release → 上传 .msi/.exe + 签名文件
       ↓
  用户本地 cc-gui
       ↓  启动时检查
  tauri-plugin-updater → 获取最新版本号 → 对比当前
       ↓  有新版本
  弹窗通知 → 用户确认 → 下载 → 安装 → 重启
```

**Cargo.toml**：
```toml
tauri-plugin-updater = "2"
```

**前端**：
```typescript
import { check } from '@tauri-apps/plugin-updater';
const update = await check();
if (update) {
  await update.downloadAndInstall();
  // 重启应用
}
```

#### 2.2 代码签名
**Windows 要求**：
- EV Code Signing Certificate（约 $300-400/年）
- 或使用 Azure Key Vault + Azure Trusted Signing（较新方案，$10-50/月）

**GitHub Actions 集成**：
```yaml
- name: Sign binary
  run: |
    signtool sign /fd SHA256 /a /f certificate.pfx /p ${{ secrets.PFX_PASSWORD }} `
      src-tauri/target/release/bundle/msi/*.msi
```

#### 2.3 更新服务器
使用 GitHub Releases 作为更新源（免费）：
- `tauri-plugin-updater` 默认支持 GitHub Releases
- 版本号从 `tauri.conf.json` 读取
- 每次 CI 发布时自动上传构建产物到 Release

### 实施阶段
| 阶段 | 内容 | 优先级 |
|------|------|--------|
| 1 | 集成 `tauri-plugin-updater`，实现基础更新检查+下载 | P0 |
| 2 | GitHub Releases 自动发布 | P0 |
| 3 | 代码签名（需购买证书） | P1 |
| 4 | 增量更新 / 静默安装 | P2 |

### 涉及文件
- `src-tauri/Cargo.toml` — 添加 updater 插件
- `src-tauri/tauri.conf.json` — updater 配置
- `src/App.vue` 或新组件 — 更新提示 UI
- `.github/workflows/release.yml` — 签名 + 发布

### 估时
- 阶段 1+2：约 2 小时
- 阶段 3：证书购买 + 配置 约半天

---

## 三、Settings 面板增强

### 目标
Settings 面板不仅是 API 配置，还要成为 **Claude Code 全部配置的中心入口**。

### 当前 Settings 面板
| 项 | 说明 |
|----|------|
| API Key | DeepSeek API 密钥 |
| Base URL | API 端点 |
| Model | 模型名称 |
| 连接测试 | 测试 API 连接 |

### 增强后
```
┌──────────────────────────────────────────────┐
│  ⚙ Settings                                  │
│                                              │
│  ┌─ API 配置 ──────────────────────────────┐ │
│  │  API Key    [························]   │ │
│  │  Base URL   [https://api.deepseek.com]   │ │
│  │  Model      [deepseek-v4-pro[1M]    ▼]   │ │
│  │  [测试连接]                               │ │
│  └──────────────────────────────────────────┘ │
│                                              │
│  ┌─ Claude Code 配置 ──────────────────────┐ │
│  │  settings.json                    [编辑] │ │
│  │  enabledPlugins: 14 个插件               │ │
│  │  permissions.defaultMode: auto           │ │
│  │  env: 12 个环境变量                       │ │
│  └──────────────────────────────────────────┘ │
│                                              │
│  ┌─ cc-gui 设置 ───────────────────────────┐ │
│  │  语言           [中文 ▼]                  │ │
│  │  Effort Level   [high ▼]                  │ │
│  │  Permission     [auto ▼]                   │ │
│  │  最大轮次        [10]                      │ │
│  │  上下文上限      1,000,000                 │ │
│  └──────────────────────────────────────────┘ │
│                                              │
│  ┌─ 关于 ──────────────────────────────────┐ │
│  │  cc-gui v0.1.0                            │ │
│  │  Tauri 2 + Vue 3 + Rust                   │ │
│  │  [检查更新]                                │ │
│  └──────────────────────────────────────────┘ │
└──────────────────────────────────────────────┘
```

### 新增功能
| 分组 | 项 | 说明 |
|------|-----|------|
| API 配置 | 连接测试结果可视化 | 显示延迟、模型列表 |
| Claude Code 配置 | settings.json 内联编辑器 | 直接编辑 `~/.claude/settings.json` |
| Claude Code 配置 | Plugins 列表 | 已启用的插件 + 一键跳转管理面板 |
| Claude Code 配置 | Permissions 摘要 | defaultMode + allow 规则数 |
| cc-gui 设置 | 语言 | zh / en 切换 |
| cc-gui 设置 | Effort 默认值 | 新会话的默认 effort |
| cc-gui 设置 | Permission 默认值 | 新会话的默认权限模式 |
| cc-gui 设置 | 最大轮次 | `--max-turns` 值 |
| cc-gui 设置 | 上下文上限 | 模型容量可自定义 |
| cc-gui 设置 | 主题 | dark / light / system |
| 关于 | 检查更新 | 手动触发更新检查 |

### 涉及文件
- `src/components/settings/SettingsPanel.vue` — 重构
- `src/locales/zh.json`, `en.json` — 新增 settings 段

### 估时
约 2-3 小时

---

## 四、一键安装 Claude Code

### 目标
全新 Windows 环境（只装了 cc-gui），启动时自动引导安装 Node.js、Git、Claude Code CLI，**让小白用户 3 分钟内完成全链路配置**。

### 用户流程
```
启动 cc-gui
    ↓
检测环境
    ├─ Node.js ✅ → 跳过
    ├─ Git ✅ → 跳过
    ├─ Claude Code CLI ❌ → 需要安装
    └─ npm ✅ → 跳过
    ↓
显示安装向导
    ┌────────────────────────────────────────┐
    │  🔧 环境检测                             │
    │                                        │
    │  ✅ Node.js v20.11.0   已安装           │
    │  ✅ Git 2.43.0          已安装           │
    │  ❌ Claude Code CLI     未安装           │
    │                                        │
    │  需要安装以下工具以使用 cc-gui：         │
    │  • Claude Code CLI                      │
    │                                        │
    │  [一键安装]   [跳过（手动安装）]         │
    └────────────────────────────────────────┘
              ↓
        安装进行中
    ┌────────────────────────────────────────┐
    │  🔧 正在安装...                          │
    │                                        │
    │  ✅ npm install -g @anthropic-ai/...    │
    │     安装完成                             │
    │                                        │
    │  ✅ 验证 claude --version               │
    │     Claude Code v2.0.0                  │
    │                                        │
    │  🎉 环境就绪！                          │
    │  [开始使用]                              │
    └────────────────────────────────────────┘
```

### 技术实现

#### 4.1 环境检测
```rust
// Rust 端：检测系统已安装的工具
#[tauri::command]
async fn detect_environment() -> Vec<EnvStatus> {
    vec![
        check("node", "--version", "Node.js"),
        check("git", "--version", "Git"),
        check("npm", "--version", "npm"),
        check("claude", "--version", "Claude Code CLI"),
    ]
}
```

#### 4.2 一键安装
```rust
// Rust 端：使用 winget 安装缺失工具
#[tauri::command]
async fn install_tool(tool: String) -> Result<String, String> {
    match tool.as_str() {
        "node" => run_cmd("winget", &["install", "OpenJS.NodeJS.LTS", "--silent"]),
        "git" => run_cmd("winget", &["install", "Git.Git", "--silent"]),
        "claude" => run_cmd("npm", &["install", "-g", "@anthropic-ai/claude-code"]),
        _ => Err("未知工具".into()),
    }
}
```

#### 4.3 安装状态机
```
未检测 → 检测中 → 缺失工具 → 安装中 → 安装完成 → 准备就绪
                                        ↓ 失败
                                     重试/跳过
```

### 边界情况处理
| 场景 | 处理 |
|------|------|
| 无网络 | 提示连接网络后重试 |
| 无管理员权限 | winget 可能失败，提示手动安装 |
| npm 安装超时 | 显示进度 + 重试按钮 |
| winget 不可用 | 降级为浏览器打开下载链接 |
| 用户已安装但 PATH 不对 | 自动搜索常见安装路径 |

### 涉及文件
- `src-tauri/src/setup.rs`（新）— 环境检测 + 安装逻辑
- `src/components/setup/SetupWizard.vue`（新）— 安装向导 UI
- `src/App.vue` — 启动时判断是否需要显示向导

### 估时
约 4-5 小时（主要是 winget 兼容性和边界情况）

---

## 实施状态

| 项目 | 状态 | 日期 |
|------|------|------|
| MCP 实时状态监控 | ✅ 已完成 | 2026-06-12 |
| Settings 面板增强 | ✅ 已完成 | 2026-06-18 |
| 一键安装 CC | ⏳ 待实施 | — |
| 自动更新 / 代码签名 | ⏳ 待实施 | — |

### Settings 面板增强 — 实际完成内容（超出原计划）

原计划只包含 cc-gui 设置项（语言/主题/effort/permission），实际完成了：

- **全项目 i18n 审计**：20 个组件模板全部迁移到 `$t()`，移除 8 个多余 `useI18n`
- **设置面板重设计**：左右两栏布局（API 配置 | cc-gui 设置）+ About footer
- **自定义下拉**：权限模式和思考深度使用自定义下拉，每条选项含图标 + 中文 + 斜体 CLI key + 描述
- **权限模式补齐**：plan 模式加入设置面板（6 选项对齐工具栏/命令面板）
- **Locale 同步修复**：App.vue 新增 watcher 将 settings.locale 同步到 i18n
- **Rust 权限同步修复**：`sync_permission_settings` 现在接受完整模式参数，bypassPermissions/dontAsk/plan 不再丢失
- **工具栏增强**：补充 bypass/dontAsk 模式识别
- **新测试**：SettingsPanel.test.ts（11 tests）
