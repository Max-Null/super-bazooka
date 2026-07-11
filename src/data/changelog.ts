/**
 * 版本更新日志（中英双语）。
 *
 * ## 发版时怎么做
 *
 * 1. 修改 package.json 和 Cargo.toml 中的版本号
 * 2. 在下方 `changelog` 数组头部新增一条 ChangelogEntry
 * 3. 更新 docs/变更记录.md
 *
 * 如果忘了第 2 步，弹窗不会显示——不报错、不崩溃，下次补上即可。
 */

import appPackage from "../../package.json";

export interface ChangelogSection {
  titleZh: string;
  titleEn: string;
  itemsZh: string[];
  itemsEn: string[];
}

export interface ChangelogEntry {
  version: string;
  date: string;
  sections: ChangelogSection[];
}

/** 当前 app 版本（与 package.json 同步） */
export const APP_VERSION: string = appPackage.version;

// 按版本降序排列（最新在第一个）
export const changelog: ChangelogEntry[] = [
  {
    version: "0.9.0",
    date: "2026-07-12",
    sections: [
      {
        titleZh: "弹框样式统一",
        titleEn: "Unified Dialog Styling",
        itemsZh: [
          "ModalShell 裸 Tailwind → 语义 class 体系（modal-shell-overlay/panel/header/body/footer/close + 4 种宽度 + 2 种垂直位置）",
          "按钮统一 btn-primary / btn-ghost，删除死代码 modal-btn-* / btn-primary-sm",
          "审批条 amber 风格 → bg-elevated + accent 竖条 + 圆角卡片，与全局 UI 色调一致",
          "状态消息从底部 → 会话区顶部 status-toast 悬浮，不遮挡审批条和 TodoPanel",
          "新增 bottom-notices 容器统一管理审批条和工作清单",
          "工作清单 TodoPanel：显示已完成项 + 序号 + 保持原始顺序 + 删除线",
          "新增语义类：badge / badge--accent / input-plain--elevated / export-btn / zen-indicator / sb-input-bar / status-toast / modal-shell-title / selection-tip",
        ],
        itemsEn: [
          "ModalShell: bare Tailwind → semantic class system (modal-shell-overlay/panel/header/body/footer/close + 4 widths + 2 positions)",
          "Buttons unified to btn-primary / btn-ghost; dead code modal-btn-* / btn-primary-sm removed",
          "Approval bar: amber → bg-elevated + accent bar + rounded card, matching global UI tone",
          "Status messages moved from bottom → top status-toast floating in chat area",
          "New bottom-notices container for approval bar + TodoPanel",
          "TodoPanel: completed items shown with strikethrough, sequence numbers, original order preserved",
          "New semantic classes: badge / input-plain--elevated / export-btn / zen-indicator / sb-input-bar / status-toast / modal-shell-title / selection-tip",
        ],
      },
      {
        titleZh: "PPTX 文字提取 + xlsx 选区",
        titleEn: "PPTX Text Extraction + xlsx Selection",
        itemsZh: [
          "PPTX 预览放弃视觉渲染，改用 JSZip 纯文字提取（段落分组 + Y 坐标排序 + 虚线分隔）",
          "xlsx 支持拖拽选区 + 发送到对话（SheetJS → 带 data-row/data-col 的 HTML 表格）",
          "PptxPreview 独立组件，xlsx 内联在 FilePreviewPanel",
          "统一选区浮动 tip（useSelectionTip composable），定位修复——取光标位 rect 而非整段包围盒",
        ],
        itemsEn: [
          "PPTX preview: dropped visual rendering, now uses JSZip text extraction (paragraph grouping + Y-sort + dashed separators)",
          "xlsx: drag-select cells + send to chat (SheetJS → HTML table with data-row/data-col)",
          "Standalone PptxPreview component; xlsx inline in FilePreviewPanel",
          "Unified selection tip (useSelectionTip composable), fixed positioning to cursor rect",
        ],
      },
      {
        titleZh: "测试工具面板 + 注释补充",
        titleEn: "Test Dev Panel + Comment Coverage",
        itemsZh: [
          "Ctrl+Shift+T 开发测试面板：12 个按钮覆盖全部弹窗/通知场景（AskUserQuestion / PlanReview / ApprovalBar / ContextUsage / Export / Rename / About / Manage / TodoWrite / Notify OK/Warn/Err）",
          "补充 9 个组件注释（ThinkingIndicator / ErrorBoundary / ChangelogDialog / ContextUsageModal / MermaidRenderer / ModeBar / DiffViewer / FilePreview / InputBar）",
          "handleDeny 改进：先清队列关闭弹窗再通知 CC，sendStdin 容错 try-catch",
        ],
        itemsEn: [
          "Ctrl+Shift+T dev test panel: 12 buttons covering all dialog/notification scenarios",
          "Added comments to 9 components (ThinkingIndicator / ErrorBoundary / ChangelogDialog etc.)",
          "handleDeny: resolve control request before sendStdin, with try-catch tolerance",
        ],
      },
    ],
  },
  {
    version: "0.8.0",
    date: "2026-07-08",
    sections: [
      {
        titleZh: "四列布局重构",
        titleEn: "4-Column Layout Refactor",
        itemsZh: [
          "新建 usePanelLayout composable 统一管理文件面板和编辑面板的拖拽列宽",
          "拖拽增加透明遮罩层防止 iframe（HTML 预览）吞鼠标事件导致拖拽卡住",
          "ResizeObserver 监听容器宽度变化，窗口缩放时自动收缩右侧面板",
          "Main 聊天区 min-width: 300px，JS 层拖拽 clamp 双向保护不被挤扁",
          "右侧面板动态上限 min(固定值, 容器宽 × 50%)，小屏自动收窄",
        ],
        itemsEn: [
          "New usePanelLayout composable unifies drag-resize logic for file panel and editor panel",
          "Transparent overlay during drag prevents iframe (HTML preview) from capturing mouse events",
          "ResizeObserver auto-shrinks right panels on window resize to prevent overflow",
          "Main chat area min-width: 300px, JS-level drag clamping protects it bidirectionally",
          "Dynamic max width min(fixed, container × 50%) for right panels on small screens",
        ],
      },
      {
        titleZh: "HTML 预览增强",
        titleEn: "HTML Preview Enhancements",
        itemsZh: [
          "预览区顶部增加宽度预设按钮栏：跟随 / 375 / 768 / 1024 / 1440 / 1920",
          "选中固定宽度后容器横向滚动，适合预览桌面端/平板/手机页面",
          "修复 CodeMirror 编辑区水平滚动条被内容挤出可视区",
        ],
        itemsEn: [
          "Width preset buttons above preview: Fit / 375 / 768 / 1024 / 1440 / 1920",
          "Fixed-width mode with horizontal scroll, ideal for previewing desktop/tablet/mobile pages",
          "Fixed CodeMirror horizontal scrollbar pushed outside visible area in edit tab",
        ],
      },
      {
        titleZh: "工具块摘要 + 会话缓存修复",
        titleEn: "Tool Summary + Session Cache Fix",
        itemsZh: [
          "Bash/PowerShell 命令行块收起时显示 description 而非完整命令原文",
          "Skill 调用块收起时显示 skill 名而非 args 原文",
          "修复 createSession 前未保存缓存导致切回时流式消息丢失",
          "修复编辑↔预览 tab 切换后编辑面板空白（离开编辑时未销毁编辑器）",
        ],
        itemsEn: [
          "Bash/PowerShell blocks show description instead of raw command when collapsed",
          "Skill blocks show skill name instead of raw args when collapsed",
          "Fixed session cache not saved before createSession, causing message loss on switch-back",
          "Fixed blank editor after edit↔preview tab switch (editor not destroyed on leaving edit tab)",
        ],
      },
      {
        titleZh: "CSS 收拢",
        titleEn: "CSS Consolidation",
        itemsZh: [
          "FilePanel 全局样式从 main.css 迁入组件 scoped style",
          "拖拽手柄 active 态统一 var(--accent-dim)，移除 !important",
        ],
        itemsEn: [
          "FilePanel global CSS moved from main.css to component scoped style",
          "Drag handle active state unified to var(--accent-dim), removed !important",
        ],
      },
    ],
  },
  {
    version: "0.7.0",
    date: "2026-07-08",
    sections: [
      {
        titleZh: "Git 适配阶段一：状态面板 + 提交 + Diff 第四列",
        titleEn: "Git Adaptation Phase 1: Status Panel + Commit + Diff 4th Column",
        itemsZh: [
          "Git 面板（FilePanel Tab 切换）：分支名 + Staged/Modified/Untracked 三段式文件列表",
          "点击文件查看 diff → 推送到第四列 360px 宽屏展示，逐行着色（增绿删红），与文件编辑器互斥",
          "行内 Stage / Unstage 按钮，底部 Commit 输入框 + Amend / 提交后推送 选项 + 独立推送按钮",
          "非 Git 仓库时一键发送指令让 CC 初始化 git（含自动安装 + .gitignore）",
          "Rust 后端：git2 库实现 6 个命令（status/diff/stage/unstage/commit/push），11 个 Rust 测试",
        ],
        itemsEn: [
          "Git panel (FilePanel tab switch): branch name + Staged/Modified/Untracked three-section file list",
          "Click file to view diff → pushed to 4th column 360px wide display, color-coded lines (green add/red del), mutually exclusive with file editor",
          "Inline Stage/Unstage buttons, commit input bar + Amend/Push-after options + standalone push button",
          "One-click CC-initiated git setup when workspace is not a repo (auto-install git + .gitignore)",
          "Rust backend: 6 git2 commands (status/diff/stage/unstage/commit/push), 11 Rust tests",
        ],
      },
      {
        titleZh: "会话分叉 + 时间线导航",
        titleEn: "Session Fork + Timeline Navigation",
        itemsZh: [
          "消息气泡新增 ✂️ 分叉按钮 → 从指定消息创建分叉会话（--fork-session）",
          "分叉会话标题自动区分（\"分叉: 消息前30字\"），点击时预检 CC 会话有效性",
          "原会话无 CC ID → 立即提示降级为普通新会话；CC 进程异常退出时聊天区可见错误提示",
          "聊天右侧时间线导航：圆点定位用户消息，悬停预览，滚动高亮当前消息（Scroll Spy）",
        ],
        itemsEn: [
          "New ✂️ fork button on message bubbles → fork session from specific message (--fork-session)",
          "Forked sessions get distinct titles (\"Fork: first 30 chars\"), pre-check CC session validity on click",
          "Missing CC session ID → immediate fallback notice; CC process crash shows visible chat error",
          "Chat right-side timeline nav: dots for user messages, hover tooltip, scroll spy highlights current",
        ],
      },
      {
        titleZh: "Markdown 渲染 + 错误提示优化",
        titleEn: "Markdown Rendering + Error UX",
        itemsZh: [
          "修复编号列表正则匹配（1.text 不跳号、排除版本号 0.5.0）",
          "修复缩进代码块识别（允许 0-3 空格缩进的 ```）",
          "列表项间空白行不再打断连续 ol/ul，相邻块自动合并",
          "API 402 Insufficient Balance 错误友好提示",
        ],
        itemsEn: [
          "Fixed numbered list regex (1.text detection, exclude version numbers like 0.5.0)",
          "Fixed indented code block recognition (allow 0-3 space-indented ```)",
          "Blank lines between list items no longer break contiguous ol/ul, adjacent blocks auto-merge",
          "Friendly error message for API 402 Insufficient Balance",
        ],
      },
    ],
  },
  {
    version: "0.6.0",
    date: "2026-07-07",
    sections: [
      {
        titleZh: "第四列文件编辑器",
        titleEn: "4th Column File Editor",
        itemsZh: [
          "对话右侧新增独立文件编辑面板，双击文件树打开，可拖拽调宽",
          "编辑 Tab（CodeMirror 6）+ 预览 Tab，Ctrl+S 保存，脏状态指示",
          "MD 编辑中选中文本 → 浮动 tip 输入修改建议 → 发送到对话（含文件路径+行号）",
          "HTML 预览点击元素 → 浮动 tip 显示 DOM 信息 → 发 HTML 片段到对话",
        ],
        itemsEn: [
          "New file editor panel to the right of chat, open with double-click, resizable",
          "Edit tab (CodeMirror 6) + Preview tab, Ctrl+S save, dirty state indicator",
          "MD selection → floating tip with suggestion input → send to chat (includes file path + line numbers)",
          "HTML preview click element → floating tip with DOM info → send HTML snippet to chat",
        ],
      },
      {
        titleZh: "智能会话管理",
        titleEn: "Smart Session Management",
        itemsZh: [
          "最新会话为空时自动跳转复用，切换工作区复用空会话，不再堆积「新会话」",
          "新建/切换会话提示改用状态 pill 而非 alert 弹窗",
        ],
        itemsEn: [
          "Auto-reuse latest empty session instead of creating duplicates, workspace switch reuses empty sessions",
          "Session notifications use status pill instead of alert dialog",
        ],
      },
      {
        titleZh: "文件操作体验优化",
        titleEn: "File Operations UX",
        itemsZh: [
          "右键删除/剪切/粘贴后，展开的子目录自动刷新，编辑器面板自动重载",
          "修复右键菜单在子目录中 contextmenu 事件冒泡导致操作失效",
          "CC 修改文件后预览区和编辑器自动刷新",
        ],
        itemsEn: [
          "Expanded subdirectories and editor panel auto-refresh after cut/delete/paste operations",
          "Fixed contextmenu event bubbling breaking right-click in subdirectories",
          "Preview and editor auto-refresh when CC modifies files",
        ],
      },
      {
        titleZh: "CSS 语义化命名收编",
        titleEn: "CSS Semantic Naming Refactor",
        itemsZh: [
          "新增 btn-ghost/btn-primary/chip/badge/status-pill 等 20+ 通用语义类",
          "BEM 命名覆盖 sb-shell ~ file-panel-* 全布局，CLAUDE.md 新增 CSS 命名硬约束",
          "工具栏宽度不足时自动换行并按组居中，滚动按钮自适应高度",
        ],
        itemsEn: [
          "20+ new semantic CSS classes: btn-ghost, btn-primary, chip, badge, status-pill, etc.",
          "BEM naming across layout from sb-shell to file-panel-*, CLAUDE.md CSS naming rules",
          "Toolbar wraps and centers on overflow, scroll button adapts to toolbar height",
        ],
      },
    ],
  },
  {
    version: "0.5.0",
    date: "2026-07-04",
    sections: [
      {
        titleZh: "文件面板右键菜单 + 文件操作",
        titleEn: "File Panel Context Menu & Operations",
        itemsZh: [
          "右键菜单 9 项：复制 / 剪切 / 粘贴 / 删除 / 重命名 / 新建文件夹 / 在资源管理器中打开 / 复制路径 / 复制文件名",
          "内联重命名 + 跨目录剪贴板共享，操作完成后自动刷新",
          "CC 修改工作区文件后自动通知文件面板刷新",
        ],
        itemsEn: [
          "9 context menu items: Copy / Cut / Paste / Delete / Rename / New Folder / Reveal in Explorer / Copy Path / Copy Name",
          "Inline rename + cross-directory clipboard, auto-refresh after operations",
          "Auto-refresh file panel when CC modifies workspace files",
        ],
      },
      {
        titleZh: "消息工具结果渲染 + 流处理修复",
        titleEn: "Tool Result Rendering & Stream Fixes",
        itemsZh: [
          "CC 工具执行结果展示在工具调用卡内部（成功 ✓ / 失败 ⚠️）",
          "修复思考内容重复（跨 tool_use 的 thinking 累积去重）",
          "修复消息内容重复渲染（v-if 链被 isEditing 打断）",
          "兼容旧版 CC system/result 事件",
        ],
        itemsEn: [
          "CC tool results displayed inside tool call cards (success ✓ / failure ⚠️)",
          "Fixed duplicate thinking content (cross-tool_use dedup)",
          "Fixed duplicate message rendering (broken v-if chain by isEditing)",
          "Legacy CC system/result event compatibility",
        ],
      },
      {
        titleZh: "文件编辑器 CodeMirror 可编辑",
        titleEn: "Editable CodeMirror in File Preview",
        itemsZh: [
          "文件预览弹窗的「编辑」tab 改为真正的 CodeMirror 6 编辑器",
          "Ctrl+S 保存、脏状态指示、关闭时未保存确认",
        ],
        itemsEn: [
          "Edit tab in file preview now uses real CodeMirror 6 editor",
          "Ctrl+S save, dirty state indicator, unsaved changes confirmation on close",
        ],
      },
      {
        titleZh: "版本更新弹窗",
        titleEn: "What's New Dialog",
        itemsZh: [
          "启动时自动检测新版本，弹出更新内容摘要",
          "中英双语，关闭后记录已读版本，同版本不重复弹出",
        ],
        itemsEn: [
          "Auto-detect new version on startup and show changelog dialog",
          "Bilingual, marks version as seen, won't show again for same version",
        ],
      },
      {
        titleZh: "设置页 UI 优化",
        titleEn: "Settings Page UI Polish",
        itemsZh: [
          "Header 和 footer 固定，中间配置区域独立滚动",
          "Footer 新增作者名 + GitHub 链接",
        ],
        itemsEn: [
          "Sticky header and footer, scrollable content area",
          "Author name + GitHub link in footer",
        ],
      },
      {
        titleZh: "启动体验优化",
        titleEn: "Startup Experience",
        itemsZh: [
          "启动画面：数据加载期间显示 Logo + 加载动画，避免空白等待",
          "并行初始化设置和会话，缩短启动时间",
          "切换会话后自动滚动到聊天底部",
        ],
        itemsEn: [
          "Splash screen with logo during data loading, no more blank wait",
          "Parallel initialization of settings and sessions for faster startup",
          "Auto-scroll to bottom when switching sessions",
        ],
      },
    ],
  },
];

/** 根据版本号查找对应的更新日志条目 */
export function findEntry(version: string): ChangelogEntry | undefined {
  return changelog.find((e) => e.version === version);
}

/** 根据当前语言返回本地化后的 section 数据 */
export function localizedSections(entry: ChangelogEntry, locale: string): { title: string; items: string[] }[] {
  const isZh = locale === "zh";
  return entry.sections.map((s) => ({
    title: isZh ? s.titleZh : s.titleEn,
    items: isZh ? s.itemsZh : s.itemsEn,
  }));
}

const STORAGE_KEY = "sb-changelog-seen";

export function getLastSeenVersion(): string {
  try {
    return localStorage.getItem(STORAGE_KEY) || "";
  } catch {
    return "";
  }
}

export function markVersionSeen(version: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, version);
  } catch { /* ignore */ }
}
