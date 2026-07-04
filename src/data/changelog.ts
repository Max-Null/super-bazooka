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
    version: "0.4.1",
    date: "2026-07-04",
    sections: [
      {
        titleZh: "文件面板右键菜单",
        titleEn: "File Panel Context Menu",
        itemsZh: [
          "右键菜单 9 项：复制 / 剪切 / 粘贴 / 删除 / 重命名 / 新建文件夹 / 在资源管理器中打开 / 复制路径 / 复制文件名",
          "内联重命名：点击文件名直接编辑，Enter 确认",
          "剪贴板跨目录共享，支持复制后粘贴到其他文件夹",
          "操作完成后自动刷新目录列表",
        ],
        itemsEn: [
          "9 context menu items: Copy / Cut / Paste / Delete / Rename / New Folder / Reveal in Explorer / Copy Path / Copy Name",
          "Inline rename: click to edit filename, Enter to confirm",
          "Cross-directory clipboard sharing",
          "Auto-refresh after file operations",
        ],
      },
      {
        titleZh: "消息工具结果渲染",
        titleEn: "Message Tool Result Rendering",
        itemsZh: [
          "CC 工具执行结果不再丢弃——展示在工具调用卡内部（成功 ✓ / 失败 ⚠️）",
          "CC 修改工作区文件后自动通知文件面板刷新",
        ],
        itemsEn: [
          "CC tool results are now displayed inside tool call cards (success ✓ / failure ⚠️)",
          "Auto-refresh file panel when CC modifies workspace files",
        ],
      },
      {
        titleZh: "流处理修复",
        titleEn: "Stream Processing Fixes",
        itemsZh: [
          "修复思考内容重复（跨 tool_use 的 thinking 累积去重）",
          "修复消息内容重复渲染（v-if 链被 isEditing 打断）",
          "兼容旧版 CC system/result 事件",
        ],
        itemsEn: [
          "Fixed duplicate thinking content (cross-tool_use dedup)",
          "Fixed duplicate message rendering (broken v-if chain by isEditing)",
          "Legacy CC system/result event compatibility",
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
