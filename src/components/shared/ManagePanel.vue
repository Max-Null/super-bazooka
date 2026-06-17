<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { readFileContent, writeFile, getClaudeDir, getWorkspaceRoot, listDir, ensureItemDescriptions, clearItemDescriptions, clearMcpDescriptions, generateMcpDescriptions, type DescriptionItem } from "@/lib/tauri-bridge";
import { connectedMcpServers } from "@/composables/useStreamProcessor";
import { useSettingsStore } from "@/stores/settings";
import ModalShell from "./ModalShell.vue";

const props = defineProps<{ open: boolean; initialTab?: string }>();
const emit = defineEmits<{ close: []; sendSlash: [text: string] }>();

const { t, locale } = useI18n();
const settingsStore = useSettingsStore();

type Tab = "plugins" | "mcp" | "skills" | "agents" | "hooks" | "memory" | "permissions" | "styles";
const tabs: { id: Tab }[] = [
  { id: "plugins" },
  { id: "mcp" },
  { id: "skills" },
  { id: "agents" },
  { id: "hooks" },
  { id: "memory" },
  { id: "permissions" },
  { id: "styles" },
];
const activeTab = ref<Tab>("plugins");

// ── 公共状态 ──
const claudeDir = ref("");
const loading = ref(false);
const error = ref("");

// 编辑器
const editingFile = ref<{ path: string; content: string } | null>(null);
const editContent = ref("");
const saved = ref(false);

// ── 初始化 ──
async function init() {
  try { claudeDir.value = await getClaudeDir(); } catch { error.value = t('manage.dirError'); }
}
watch(() => props.open, (v) => {
  if (!v) return;
  init().then(() => {
    const tab = (props.initialTab || "plugins") as Tab;
    switchTab(tab);
  });
});

// ── 文件编辑器 ──
async function openEditor(path: string) {
  loading.value = true;
  try {
    const content = await readFileContent(path);
    editingFile.value = { path, content };
    editContent.value = content;
    saved.value = false;
  } catch { error.value = t('manage.readError'); }
  loading.value = false;
}
async function saveEdit() {
  if (!editingFile.value) return;
  await writeFile(editingFile.value.path, editContent.value);
  editingFile.value.content = editContent.value;
  saved.value = true;
  setTimeout(() => saved.value = false, 2000);
}

// ── 各 Tab 数据 ──
interface Item { name: string; detail?: string; desc?: string | null; path?: string; enabled?: boolean; disabled?: boolean; _key?: string }
type FlatItem = Item | { type: "group"; label: string };
const items = ref<Item[]>([]);
const treeItems = ref<FlatItem[]>([]);
const collapsedGroups = ref<Set<string>>(new Set());
const settingsRaw = ref("");

function toggleGroup(label: string) {
  if (collapsedGroups.value.has(label)) collapsedGroups.value.delete(label);
  else collapsedGroups.value.add(label);
}

function isItemVisible(index: number): boolean {
  for (let j = index - 1; j >= 0; j--) {
    const prev = treeItems.value[j];
    if ("type" in prev && prev.type === "group") {
      return !collapsedGroups.value.has((prev as any).label);
    }
  }
  return true;
}
function itemVisibleKey(index: number): string {
  const g = findGroup(index);
  return g ? `${g}_${collapsedGroups.value.has(g)}` : "root";
}
function findGroup(index: number): string | null {
  for (let j = index - 1; j >= 0; j--) {
    const prev = treeItems.value[j];
    if ("type" in prev && prev.type === "group") return (prev as any).label;
  }
  return null;
}

async function loadJSON(key: string, transform: (data: any) => Item[]) {
  loading.value = true; error.value = "";
  try {
    const raw = await readFileContent(`${claudeDir.value}/settings.json`);
    settingsRaw.value = raw;
    items.value = transform(JSON.parse(raw));
  } catch { items.value = []; }
  loading.value = false;
}

async function togglePlugin(name: string) {
  try {
    const data = JSON.parse(settingsRaw.value);
    const plugins = data.enabledPlugins || {};
    plugins[name] = !plugins[name];
    data.enabledPlugins = plugins;
    const newRaw = JSON.stringify(data, null, 2) + "\n";
    await writeFile(`${claudeDir.value}/settings.json`, newRaw);
    settingsRaw.value = newRaw;
    const it = items.value.find(i => i._key === name);
    if (it) it.enabled = plugins[name];
  } catch {}
}

async function removePlugin(name: string) {
  try {
    const data = JSON.parse(settingsRaw.value);
    const plugins = data.enabledPlugins || {};
    delete plugins[name];
    data.enabledPlugins = plugins;
    const newRaw = JSON.stringify(data, null, 2) + "\n";
    await writeFile(`${claudeDir.value}/settings.json`, newRaw);
    settingsRaw.value = newRaw;
    items.value = items.value.filter(i => i._key !== name);
  } catch {}
}

function addPlugin() {
  emit("sendSlash", "/plugin install ");
  emit("close");
}

async function loadDir(subdir: string, filter?: (f: any) => boolean, map?: (f: any) => Item) {
  loading.value = true; error.value = "";
  try {
    const path = `${claudeDir.value}/${subdir}`;
    const files = await listDir(path);
    let result = files;
    if (filter) result = result.filter(filter);
    if (map) items.value = result.map(map);
    else items.value = result.map(f => ({ name: f.name, path: f.path }));
  } catch { items.value = []; }
  loading.value = false;
}

async function loadMemory() {
  loading.value = true; error.value = "";
  const result: FlatItem[] = [];

  const globalFiles = [
    { path: `${claudeDir.value}/CLAUDE.md`, label: "CLAUDE.md (全局)" },
    { path: `${claudeDir.value}/MEMORY.md`, label: "MEMORY.md (全局索引)" },
  ];
  const globalItems: Item[] = [];
  for (const gf of globalFiles) {
    try {
      await readFileContent(gf.path);
      globalItems.push({ name: gf.label, path: gf.path });
    } catch {}
  }
  if (globalItems.length > 0) {
    result.push({ type: "group", label: t('manage.globalMemory') });
    for (const it of globalItems) result.push(it);
  }

  try {
    const projectsDir = `${claudeDir.value}/projects`;
    const projectDirs = await listDir(projectsDir);
    for (const proj of projectDirs) {
      if (!proj.is_dir) continue;
      const projName = proj.name
        .replace(/^h--maxnull-workstation-?/, "")
        .replace(/^-+/, "")
        .replace(/-/g, " ")
        .trim() || proj.name;
      try {
        const memDir = `${proj.path}/memory`;
        const files = await listDir(memDir);
        const mdFiles = files.filter(f => f.name.endsWith(".md"));
        if (mdFiles.length === 0) continue;
        result.push({ type: "group", label: `📁 ${projName}` });
        for (const f of mdFiles) {
          result.push({ name: f.name, path: f.path });
        }
      } catch {}
    }
  } catch {}

  try {
    const pd = await listDir(`${claudeDir.value}/projects`);
    for (const d of pd) {
      if (!d.is_dir || !d.name.includes("cc-gui") || d.name.includes("src-tauri")) continue;
      const candidates = [
        `H:/MaxNull/WorkStation/cc-gui/CLAUDE.md`,
      ];
      for (const lp of candidates) {
        try {
          await readFileContent(lp);
          result.push({ type: "group", label: t('manage.localMemory') });
          result.push({ name: "CLAUDE.md", path: lp });
          break;
        } catch {}
      }
      break;
    }
  } catch {}

  treeItems.value = result;
  loading.value = false;
}

// ═══════════════════ MCP 管理 ═══════════════════

// MCP 连接类型 → 中文标签映射
function getMcpTypeLabel(rawType: string | undefined): string {
  switch (rawType) {
    case "stdio": return t('manage.mcpTypeStdio');
    case "sse": return t('manage.mcpTypeSse');
    case "http": return t('manage.mcpTypeHttp');
    case "websocket": return t('manage.mcpTypeWs');
    default: return t('manage.mcpTypeStdio'); // 默认 stdio
  }
}

// 尝试从 SKILL.md / plugin.json 等文件中解析 description 字段
function parseDescription(raw: string): string | null {
  // YAML frontmatter: ---\ndescription: "..."
  const m = raw.match(/^---\s*\n(.*?)\n---/s);
  if (m) {
    const fm = m[1];
    const dm = fm.match(/^description:\s*(.+)$/m);
    if (dm) {
      // 去掉首尾引号
      let desc = dm[1].trim();
      if ((desc.startsWith('"') && desc.endsWith('"')) || (desc.startsWith("'") && desc.endsWith("'"))) {
        desc = desc.slice(1, -1);
      }
      // 完整返回，不做截断；显示时用 CSS line-clamp
      return desc;
    }
  }
  // JSON: { "description": "..." }
  try {
    const json = JSON.parse(raw);
    if (json.description && typeof json.description === "string") {
      return json.description;
    }
  } catch {}
  return null;
}

// 从 MCP 配置中提取 description 字段（如果有的话）。没有则返回 null，由 AI 生成补全。
function extractMcpDesc(cfg: any): string | null {
  if (cfg.description && typeof cfg.description === "string") return cfg.description;
  return null;
}

function extractMcpServers(data: any): Record<string, any> {
  if (data.mcpServers && typeof data.mcpServers === "object") return data.mcpServers;
  const servers: Record<string, any> = {};
  for (const [k, v] of Object.entries(data)) {
    if (typeof v === "object" && v !== null && ("type" in v || "command" in v)) {
      servers[k] = v;
    }
  }
  return servers;
}

async function scanMcpFile(mcpPath: string, result: Item[], configured: Set<string>, sourceLabel: string) {
  try {
    const raw = await readFileContent(mcpPath);
    const data = JSON.parse(raw);
    const servers = extractMcpServers(data);

    for (const [name, cfg] of Object.entries(servers)) {
      if (configured.has(name)) continue;
      configured.add(name);
      const rawType = (cfg as any).type as string | undefined;
      const typeLabel = getMcpTypeLabel(rawType);
      result.push({
        name,
        detail: `${sourceLabel} · ${typeLabel}`,
        desc: extractMcpDesc(cfg),
        enabled: true,
      });
    }
  } catch {}
}

async function loadMCP() {
  loading.value = true; error.value = "";
  const result: Item[] = [];
  const configured = new Set<string>();

  // 1) 静态 MCP 配置源
  // ~/.claude.json = Claude Code 官方用户级 MCP 配置文件
  // ~/.claude/settings.json 的 mcpServers 字段会被 CLI 静默忽略！
  const homeDir = claudeDir.value.replace(/[\/\\]\.claude$/, '');
  const configPaths: string[] = [
    `${homeDir}/.claude.json`,           // 用户级 MCP 配置（← 主要的）
    `${claudeDir.value}/.mcp.json`,      // 兼容旧版
  ];
  try {
    const workspaceRoot = await getWorkspaceRoot();
    if (workspaceRoot) configPaths.push(`${workspaceRoot}/.mcp.json`);
  } catch {}

  for (const path of configPaths) {
    await scanMcpFile(path, result, configured, t('manage.mcpSourceUser'));
  }

  // 2) 插件 .mcp.json 文件（只扫描已启用的插件）
  let enabledPlugins: Record<string, boolean> = {};
  try {
    const raw = await readFileContent(`${claudeDir.value}/settings.json`);
    const data = JSON.parse(raw);
    enabledPlugins = data.enabledPlugins || {};
  } catch {}

  try {
    const mpDir = `${claudeDir.value}/plugins/marketplaces`;
    const mps = await listDir(mpDir);
    for (const mp of mps) {
      if (!mp.is_dir) continue;
      for (const sub of ["external_plugins", "plugins"]) {
        try {
          const plugins = await listDir(`${mp.path}/${sub}`);
          for (const plug of plugins) {
            if (!plug.is_dir) continue;
            // 只扫描已启用的插件
            const key = `${plug.name}@${mp.name}`;
            if (!enabledPlugins[key]) continue;
            await scanMcpFile(
              `${plug.path}/.mcp.json`, result, configured,
              `${t('manage.mcpSourcePlugin')} · ${plug.name}`,
            );
          }
        } catch {}
      }
    }
  } catch {}

  // 3) 读取禁用列表
  const disabledSet = new Set<string>();
  try {
    const claudeRaw = await readFileContent(`${homeDir}/.claude.json`);
    const claudeData = JSON.parse(claudeRaw);
    let workspaceRoot = "";
    try { workspaceRoot = await getWorkspaceRoot(); } catch {}
    const proj = workspaceRoot ? claudeData?.projects?.[workspaceRoot] : null;
    if (proj?.disabledMcpServers) {
      for (const s of proj.disabledMcpServers) disabledSet.add(s);
    }
  } catch {}

  // 4) 运行时连接状态 + 禁用标记
  const connected = new Set(connectedMcpServers.value);
  for (const it of result) {
    it.enabled = connected.has(it.name);
    it.disabled = disabledSet.has(it.name);
  }
  for (const srv of connected) {
    if (!configured.has(srv)) {
      result.push({ name: srv, detail: t('manage.mcpSourceRuntime'), enabled: true, disabled: disabledSet.has(srv) });
    }
  }

  items.value = result;
  loading.value = false;
}

async function toggleMcp(name: string) {
  const homeDir = claudeDir.value.replace(/[\/\\]\.claude$/, '');
  const path = `${homeDir}/.claude.json`;
  let workspaceRoot = "";
  try { workspaceRoot = await getWorkspaceRoot(); } catch {}
  if (!workspaceRoot) return;

  try {
    const raw = await readFileContent(path);
    const data = JSON.parse(raw);
    if (!data.projects) data.projects = {};
    if (!data.projects[workspaceRoot]) data.projects[workspaceRoot] = {};
    if (!data.projects[workspaceRoot].disabledMcpServers) data.projects[workspaceRoot].disabledMcpServers = [];

    const list: string[] = data.projects[workspaceRoot].disabledMcpServers;
    const idx = list.indexOf(name);
    if (idx >= 0) {
      list.splice(idx, 1); // 启用
    } else {
      list.push(name); // 禁用
    }
    await writeFile(path, JSON.stringify(data, null, 2));

    // 更新 UI
    const it = items.value.find(i => i.name === name);
    if (it) it.disabled = idx < 0; // 原来是启用的→现在禁用
  } catch (e) {
    error.value = String(e);
  }
}

// ═══════════════════ Skills 管理（含描述） ═══════════════════

async function loadSkills() {
  loading.value = true; error.value = "";
  try {
    const path = `${claudeDir.value}/skills`;
    const dirs = await listDir(path);
    const result: Item[] = [];
    for (const d of dirs) {
      if (!d.is_dir) continue;
      let desc: string | null = null;
      try {
        const skillMd = await readFileContent(`${d.path}/SKILL.md`);
        desc = parseDescription(skillMd);
      } catch {}
      result.push({ name: d.name, path: d.path, desc });
    }
    items.value = result;
  } catch { items.value = []; }
  loading.value = false;
}

// ═══════════════════ Plugins 管理（含描述） ═══════════════════

async function loadPlugins() {
  loading.value = true; error.value = "";
  try {
    const raw = await readFileContent(`${claudeDir.value}/settings.json`);
    settingsRaw.value = raw;
    const data = JSON.parse(raw);
    const plugins = data.enabledPlugins || {};
    const result: Item[] = [];

    for (const [key, enabled] of Object.entries(plugins)) {
      // 尝试从多个可能的路径读取 plugin.json 获取描述
      let desc: string | null = null;
      // key 格式: "name@marketplace" → 查找对应目录
      const atIdx = key.lastIndexOf("@");
      const plugName = atIdx > 0 ? key.slice(0, atIdx) : key;
      const marketplace = atIdx > 0 ? key.slice(atIdx + 1) : "claude-plugins-official";

      // 防路径穿越：只允许字母、数字、连字符、下划线、点号
      const safeNameRE = /^[a-zA-Z0-9._-]+$/;
      if (!safeNameRE.test(plugName) || !safeNameRE.test(marketplace)) {
        result.push({ name: key, enabled: !!enabled, _key: key });
        continue;
      }

      const candidateDirs = [
        `${claudeDir.value}/plugins/marketplaces/${marketplace}/external_plugins/${plugName}`,
        `${claudeDir.value}/plugins/marketplaces/${marketplace}/plugins/${plugName}`,
      ];
      for (const dir of candidateDirs) {
        try {
          const pjRaw = await readFileContent(`${dir}/.claude-plugin/plugin.json`);
          desc = parseDescription(pjRaw);
          if (desc) break;
        } catch {}
      }

      result.push({
        name: key,
        desc,
        enabled: !!enabled,
        _key: key,
      });
    }
    items.value = result;
  } catch { items.value = []; }
  loading.value = false;
}

// ═══════════════════ Agents 管理（含描述） ═══════════════════

async function loadAgents() {
  loading.value = true; error.value = "";
  try {
    const path = `${claudeDir.value}/agents`;
    const files = await listDir(path);
    const result: Item[] = [];
    for (const f of files) {
      if (!f.name.endsWith(".md")) continue;
      let desc: string | null = null;
      try {
        const content = await readFileContent(f.path);
        desc = parseDescription(content);
      } catch {}
      result.push({ name: f.name, path: f.path, desc });
    }
    items.value = result;
  } catch { items.value = []; }
  loading.value = false;
}

// ── 描述翻译（调 Rust → DB 缓存 + DeepSeek API）──
const translateError = ref("");

async function enrichDescriptions() {
  translateError.value = "";
  const needTranslate: DescriptionItem[] = [];
  for (const it of items.value) {
    if (it.desc) {
      needTranslate.push({
        item_type: activeTab.value,
        name: it.name,
        desc_en: it.desc,
      });
    }
  }
  if (needTranslate.length === 0) return;

  try {
    const enriched = await ensureItemDescriptions(
      needTranslate,
      settingsStore.apiKey,
      settingsStore.baseUrl,
    );
    const map = new Map(enriched.map(e => [e.name, e]));
    for (const it of items.value) {
      const e = map.get(it.name);
      if (e) {
        it.desc = locale.value === "zh"
          ? (e.desc_zh || e.desc_en)
          : (e.desc_en || e.desc_zh);
      }
    }
  } catch (err) {
    translateError.value = String(err);
  }
}

// ── MCP 描述生成（AI 联网查询）──
async function enrichMcpDescriptions() {
  const needQuery: string[] = [];
  for (const it of items.value) {
    if (!it.desc) needQuery.push(it.name);
  }
  if (needQuery.length === 0) return;

  // 先设占位符，避免空白闪烁
  for (const it of items.value) {
    if (!it.desc) it.desc = "…";
  }

  translateError.value = "";
  try {
    const generated = await generateMcpDescriptions(
      needQuery,
      settingsStore.apiKey,
      settingsStore.baseUrl,
    );
    const map = new Map(generated.map(e => [e.name, e.desc_zh]));
    for (const it of items.value) {
      const zh = map.get(it.name);
      if (zh) {
        it.desc = zh;
      } else if (it.desc === "…") {
        it.desc = null;
      }
    }
  } catch (err) {
    translateError.value = String(err);
    for (const it of items.value) {
      if (it.desc === "…") it.desc = null;
    }
  }
}

// 清空 MCP 描述缓存并重新生成
async function retranslateMCP() {
  translateError.value = "";
  try { await clearMcpDescriptions(); } catch {}
  for (const it of items.value) it.desc = null;
  await enrichMcpDescriptions();
}

// 清空缓存并重新翻译
async function retranslateAll() {
  translateError.value = "";
  try {
    await clearItemDescriptions();
  } catch {}
  await enrichDescriptions();
}

// ── Tab 切换 ──
function switchTab(t: Tab) {
  activeTab.value = t;
  items.value = [];
  switch (t) {
    case "plugins": loadPlugins().then(enrichDescriptions); break;
    case "mcp": loadMCP().then(enrichDescriptions).then(enrichMcpDescriptions); break;
    case "skills": loadSkills().then(enrichDescriptions); break;
    case "agents": loadAgents().then(enrichDescriptions); break;
    case "hooks":
      loadJSON("hooks", data =>
        Object.entries(data.hooks || {}).map(([event, cfg]: [string, any]) => ({
          name: `${event} → ${cfg.matcher || cfg.command || "(rule)"}`,
          enabled: !cfg.disabled,
        }))
      ); break;
    case "memory": loadMemory(); break;
    case "permissions":
      loadJSON("permissions", data => {
        const p = data.permissions || {};
        const result: Item[] = [];
        if (p.defaultMode) result.push({ name: `defaultMode: ${p.defaultMode}`, enabled: true });
        return result;
      }); break;
    case "styles":
      loadJSON("outputStyles", data => {
        const os = data.outputStyles || data.outputStyle || {};
        return Object.entries(os).map(([k, v]) => ({ name: `${k}: ${typeof v === 'string' ? v.slice(0, 80) : JSON.stringify(v).slice(0, 80)}` }));
      }); break;
  }
}
</script>

<template>
  <ModalShell :open="open" size="lg" @close="emit('close')">
    <template #header>
      <div class="flex flex-col flex-1">
        <div class="flex items-center gap-2">
          <span class="text-sm font-semibold" :style="{ color: 'var(--text-bright)' }">{{ $t('manage.title') }}</span>
          <span class="text-[11px] font-mono" :style="{ color: 'var(--text-muted)' }">{{ claudeDir || $t('manage.loading') }}</span>
        </div>
        <div class="flex flex-wrap gap-0.5 mt-1.5">
          <button v-for="tab in tabs" :key="tab.id" @click="switchTab(tab.id)" class="px-2.5 py-1 rounded text-xs transition-colors" :style="{ background: activeTab === tab.id ? 'var(--accent-glow)' : 'transparent', color: activeTab === tab.id ? 'var(--accent)' : 'var(--text-muted)' }">{{ $t('manage.' + tab.id) }}</button>
        </div>
      </div>
    </template>

    <!-- 编辑器模式 -->
    <template v-if="editingFile">
      <div class="flex items-center gap-2 mb-2">
        <button @click="editingFile = null" class="text-xs px-2 py-0.5 rounded hover:bg-[var(--bg-hover)]" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.back') }}</button>
        <span class="text-[11px] font-mono truncate" :style="{ color: 'var(--text-secondary)' }">{{ editingFile.path }}</span>
        <div class="flex-1"></div>
        <button @click="saveEdit" class="px-2.5 py-1 rounded text-xs font-medium" :style="{ background: saved ? 'var(--accent-dim)' : 'var(--accent)', color: 'var(--bg-root)' }">{{ saved ? $t('manage.saved') : $t('manage.save') }}</button>
      </div>
      <textarea v-model="editContent" class="w-full rounded-md p-3 text-xs font-mono leading-relaxed resize-none outline-none" :style="{ background: 'var(--bg-elevated)', color: 'var(--text-primary)', border: '1px solid var(--border-dim)', minHeight: '340px' }" rows="18"></textarea>
    </template>

    <!-- 主视图 -->
    <template v-else>
      <div v-if="loading" class="text-xs py-4 text-center" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.loading') }}</div>
      <div v-else-if="error" class="text-xs py-4 text-center" :style="{ color: 'var(--coral)' }">{{ error }}</div>

      <!-- Memory 树形列表 -->
      <div v-else-if="activeTab === 'memory'" class="space-y-0">
        <template v-for="(it, i) in treeItems" :key="i">
          <div v-if="'type' in it && it.type === 'group'"
            @click="toggleGroup((it as any).label)"
            class="px-3 pt-3 pb-1 text-[10px] font-semibold uppercase tracking-wider select-none flex items-center gap-1.5 cursor-pointer hover:text-[var(--text-secondary)] transition-colors"
            :style="{ color: 'var(--text-muted)' }"
          >
            <span class="text-[9px] w-3 text-center">{{ collapsedGroups.has((it as any).label) ? '▸' : '▾' }}</span>
            {{ (it as any).label }}
          </div>
          <div v-else
            v-show="isItemVisible(i)"
            :key="itemVisibleKey(i)"
            class="flex items-center gap-2 pl-8 pr-3 py-1.5 rounded text-xs group hover:bg-[var(--bg-hover)] cursor-pointer transition-colors"
            :style="{ color: 'var(--text-secondary)' }"
            @click="openEditor((it as Item).path!)"
          >
            <span class="text-[11px]">🧠</span>
            <span class="font-mono flex-1 truncate">{{ (it as Item).name }}</span>
            <button class="text-[10px] px-1.5 py-0.5 rounded opacity-0 group-hover:opacity-100 transition-opacity shrink-0" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.edit') }}</button>
          </div>
        </template>
        <div v-if="treeItems.length === 0" class="text-xs py-4 text-center" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.noMemory') }}</div>
      </div>

      <!-- 其他 Tab：带描述的平铺列表 -->
      <div v-else class="space-y-0.5">
        <div v-for="it in items" :key="it.name"
          class="px-3 py-2 rounded text-xs group"
          :style="{ color: it.enabled === false ? 'var(--text-muted)' : 'var(--text-secondary)' }"
        >
          <div class="flex items-center gap-2">
            <span class="w-1.5 h-1.5 rounded-full shrink-0"
              :style="{
                background: activeTab === 'mcp'
                  ? (it.disabled ? 'var(--coral)' : it.enabled ? 'var(--accent)' : 'var(--border-default)')
                  : (it.enabled === false ? 'var(--border-bright)' : it.enabled ? 'var(--accent)' : 'var(--border-default)')
              }"
            ></span>
            <span class="font-mono flex-1 truncate">{{ it.name }}</span>
            <span v-if="it.detail" class="text-[10px] shrink-0" :style="{ color: 'var(--text-muted)' }">{{ it.detail }}</span>
            <!-- MCP 状态标签 + 开关 -->
            <template v-if="activeTab === 'mcp'">
              <span class="text-[10px] shrink-0" :style="{ color: it.disabled ? 'var(--coral)' : it.enabled ? 'var(--accent)' : 'var(--text-muted)' }">
                {{ it.disabled ? $t('manage.mcpDisabled') : it.enabled ? $t('manage.mcpConnected') : $t('manage.mcpOffline') }}
              </span>
              <button @click="toggleMcp(it.name)"
                class="text-[10px] px-2 py-0.5 rounded transition-colors shrink-0"
                :style="{ background: it.disabled ? 'var(--accent-glow)' : 'var(--coral-glow)', color: it.disabled ? 'var(--accent)' : 'var(--coral)' }"
              >{{ it.disabled ? $t('manage.mcpToggleEnable') : $t('manage.mcpToggleDisable') }}</button>
            </template>
            <template v-if="activeTab === 'plugins' && it._key">
              <button @click="togglePlugin(it._key)"
                class="text-[10px] px-2 py-0.5 rounded transition-colors shrink-0"
                :style="{ background: it.enabled ? 'var(--accent-glow)' : 'var(--bg-hover)', color: it.enabled ? 'var(--accent)' : 'var(--text-muted)' }"
              >{{ it.enabled ? $t('manage.disable') : $t('manage.enable') }}</button>
              <button @click="removePlugin(it._key!)"
                class="text-[10px] px-1.5 py-0.5 rounded hover:bg-[var(--bg-hover)] transition-colors shrink-0 opacity-0 group-hover:opacity-100"
                :style="{ color: 'var(--coral)' }" :title="$t('manage.deletePlugin')"
              >✕</button>
            </template>
            <button v-if="it.path && activeTab !== 'skills' && activeTab !== 'plugins' && activeTab !== 'mcp'" @click="openEditor(it.path)" class="text-[10px] px-1.5 py-0.5 rounded hover:bg-[var(--bg-hover)] opacity-0 group-hover:opacity-100 transition-opacity shrink-0" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.edit') }}</button>
            <button v-if="it.path && (activeTab === 'skills' || activeTab === 'agents')" @click="openEditor(it.path + (activeTab === 'skills' ? '/SKILL.md' : ''))" class="text-[10px] px-1.5 py-0.5 rounded hover:bg-[var(--bg-hover)] opacity-0 group-hover:opacity-100 transition-opacity shrink-0" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.view') }}</button>
          </div>
          <div v-if="it.desc" class="text-[10px] mt-1 leading-relaxed pl-5 line-clamp-2" :style="{ color: 'var(--text-muted)' }" :title="it.desc">{{ it.desc }}</div>
        </div>
        <div v-if="items.length === 0 && activeTab !== 'plugins'" class="text-xs py-4 text-center" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.noData') }}</div>
      </div>
    </template>

    <template #footer>
      <div v-if="translateError" class="text-[10px] mb-1.5" :style="{ color: 'var(--coral)' }">{{ $t('manage.translateError') }}: {{ translateError }}</div>
      <div v-if="activeTab === 'plugins'" class="text-[10px] leading-relaxed" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.pluginsDesc') }}</div>
      <div v-if="activeTab === 'mcp'" class="text-[10px] leading-relaxed" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.mcpDesc') }}</div>
      <div v-if="activeTab === 'mcp'" class="pt-2">
        <button @click="retranslateMCP" class="w-full text-xs px-3 py-1.5 rounded border border-dashed transition-colors hover:bg-[var(--bg-hover)]" :style="{ color: 'var(--text-muted)', borderColor: 'var(--border-dim)' }">{{ $t('manage.retranslate') }}</button>
      </div>
      <div v-if="activeTab === 'skills'" class="text-[10px] leading-relaxed" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.skillsDesc') }}</div>
      <div v-if="activeTab === 'agents'" class="text-[10px] leading-relaxed" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.agentsDesc') }}</div>
      <div v-if="activeTab === 'styles'" class="text-[10px] leading-relaxed" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.stylesDesc') }}</div>
      <div v-if="activeTab === 'plugins'" class="pt-2">
        <button @click="addPlugin" class="w-full text-xs px-3 py-1.5 rounded border border-dashed transition-colors hover:bg-[var(--bg-hover)]" :style="{ color: 'var(--text-muted)', borderColor: 'var(--border-dim)' }">{{ $t('manage.addPlugin') }}</button>
      </div>
    </template>
  </ModalShell>
</template>
