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
