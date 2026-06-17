<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { readFileContent, writeFile, getClaudeDir, listDir } from "@/lib/tauri-bridge";
import { connectedMcpServers } from "@/composables/useStreamProcessor";
import ModalShell from "./ModalShell.vue";

const props = defineProps<{ open: boolean; initialTab?: string }>();
const emit = defineEmits<{ close: []; sendSlash: [text: string] }>();

const { t } = useI18n();

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
interface Item { name: string; detail?: string; path?: string; enabled?: boolean; _key?: string }
type FlatItem = Item | { type: "group"; label: string };
const items = ref<Item[]>([]);
const treeItems = ref<FlatItem[]>([]); // Memory 专用的树形结构
const collapsedGroups = ref<Set<string>>(new Set()); // 折叠的分组
const settingsRaw = ref(""); // 缓存的 settings.json 原始内容

function toggleGroup(label: string) {
  if (collapsedGroups.value.has(label)) collapsedGroups.value.delete(label);
  else collapsedGroups.value.add(label);
}

// 计算每个 treeItem 是否显示（受父分组折叠状态影响）
function isItemVisible(index: number): boolean {
  for (let j = index - 1; j >= 0; j--) {
    const prev = treeItems.value[j];
    if ("type" in prev && prev.type === "group") {
      return !collapsedGroups.value.has((prev as any).label);
    }
  }
  return true;
}
// 确保响应式触发
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

// 切换 settings.json 中 enabledPlugins 的某个插件
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

// 删除插件：从 enabledPlugins 中移除
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

// 新增插件：注入 /plugin install 到聊天输入框，用户补全插件名后发送
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

  // ── 1. 全局级：~/.claude/CLAUDE.md, ~/.claude/MEMORY.md ──
  const globalFiles = [
    { path: `${claudeDir.value}/CLAUDE.md`, label: "CLAUDE.md (全局)" },
    { path: `${claudeDir.value}/MEMORY.md`, label: "MEMORY.md (全局索引)" },
  ];
  const globalItems: Item[] = [];
  for (const gf of globalFiles) {
    try {
      await readFileContent(gf.path); // 验证可读
      globalItems.push({ name: gf.label, path: gf.path });
    } catch {}
  }
  if (globalItems.length > 0) {
    result.push({ type: "group", label: t('manage.globalMemory') });
    for (const it of globalItems) result.push(it);
  }

  // ── 2. 项目级：~/.claude/projects/<slug>/memory/*.md ──
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

  // ── 3. 本地级：工作区根目录的 CLAUDE.md ──
  try {
    // 遍历项目目录，找到对应的工作区路径
    const pd = await listDir(`${claudeDir.value}/projects`);
    for (const d of pd) {
      if (!d.is_dir || !d.name.includes("cc-gui") || d.name.includes("src-tauri")) continue;
      // 反向推导工作区路径并尝试读取 CLAUDE.md
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

async function loadMCP() {
  loading.value = true; error.value = "";
  const result: Item[] = [];
  // 收集所有静态配置的 MCP 服务器
  const configured = new Set<string>();
  const candidates = [`${claudeDir.value}/settings.json`];
  for (const path of candidates) {
    try {
      const raw = await readFileContent(path);
      const data = JSON.parse(raw);
      const servers = data.mcpServers || {};
      for (const [name, cfg] of Object.entries(servers)) {
        configured.add(name);
        result.push({ name, detail: (cfg as any).type || t('manage.mcpDetail'), enabled: true });
      }
    } catch {}
  }
  // 标记运行时连接状态
  const connected = new Set(connectedMcpServers.value);
  for (const it of result) {
    it.enabled = connected.has(it.name); // enabled → 🟢 已连接
  }
  // 添加仅在运行时存在但未在静态配置中的 MCP 服务器
  for (const srv of connected) {
    if (!configured.has(srv)) {
      result.push({ name: srv, detail: t('manage.mcpDiscovered'), enabled: true });
    }
  }
  items.value = result;
  loading.value = false;
}

// ── Tab 切换 ──
function switchTab(t: Tab) {
  activeTab.value = t;
  items.value = []; // 切换时清空旧数据
  switch (t) {
    case "plugins":
      loadJSON("enabledPlugins", data =>
        Object.entries(data.enabledPlugins || {}).map(([k, v]) => ({ name: k, enabled: !!v, _key: k }))
      ); break;
    case "mcp": loadMCP(); break;
    case "skills": loadDir("skills", f => f.is_dir, f => ({ name: f.name, path: f.path })); break;
    case "agents": loadDir("agents", f => f.name.endsWith(".md"), f => ({ name: f.name, path: f.path })); break;
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
        // 额外检查 .mcp.json 里的权限
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
      <span class="text-sm font-semibold" :style="{ color: 'var(--text-bright)' }">{{ $t('manage.title') }}</span>
      <span class="text-[11px] font-mono ml-2" :style="{ color: 'var(--text-muted)' }">{{ claudeDir || $t('manage.loading') }}</span>
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
      <!-- Tab -->
      <div class="flex flex-wrap gap-0.5 mb-3 -mt-1">
        <button v-for="tab in tabs" :key="tab.id" @click="switchTab(tab.id)" class="px-2.5 py-1 rounded text-xs transition-colors" :style="{ background: activeTab === tab.id ? 'var(--accent-glow)' : 'transparent', color: activeTab === tab.id ? 'var(--accent)' : 'var(--text-muted)' }">{{ $t('manage.' + tab.id) }}</button>
      </div>

      <div v-if="loading" class="text-xs py-4 text-center" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.loading') }}</div>
      <div v-else-if="error" class="text-xs py-4 text-center" :style="{ color: 'var(--coral)' }">{{ error }}</div>

      <!-- Memory 树形列表 — 可折叠分组 + 缩进文件 -->
      <div v-else-if="activeTab === 'memory'" class="space-y-0">
        <template v-for="(it, i) in treeItems" :key="i">
          <!-- 分组标题：可点击折叠 -->
          <div v-if="'type' in it && it.type === 'group'"
            @click="toggleGroup((it as any).label)"
            class="px-3 pt-3 pb-1 text-[10px] font-semibold uppercase tracking-wider select-none flex items-center gap-1.5 cursor-pointer hover:text-[var(--text-secondary)] transition-colors"
            :style="{ color: 'var(--text-muted)' }"
          >
            <span class="text-[9px] w-3 text-center">{{ collapsedGroups.has((it as any).label) ? '▸' : '▾' }}</span>
            {{ (it as any).label }}
          </div>
          <!-- 分组内文件 — 缩进，折叠时隐藏 -->
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

      <!-- 其他 Tab 平铺列表 -->
      <div v-else class="space-y-0.5">
        <div v-for="it in items" :key="it.name"
          class="flex items-center gap-2 px-3 py-1.5 rounded text-xs group"
          :style="{ color: it.enabled === false ? 'var(--text-muted)' : 'var(--text-secondary)' }"
        >
          <span class="w-1.5 h-1.5 rounded-full shrink-0" :style="{ background: it.enabled === false ? 'var(--border-bright)' : it.enabled ? 'var(--accent)' : 'var(--border-default)' }"></span>
          <span class="font-mono flex-1 truncate">{{ it.name }}</span>
          <span v-if="it.detail" class="text-[10px] shrink-0" :style="{ color: 'var(--text-muted)' }">{{ it.detail }}</span>
          <!-- Plugins 专用：启停开关 + 删除 -->
          <template v-if="activeTab === 'plugins' && it._key">
            <button @click="togglePlugin(it._key)"
              class="text-[10px] px-2 py-0.5 rounded transition-colors shrink-0"
              :style="{
                background: it.enabled ? 'var(--accent-glow)' : 'var(--bg-hover)',
                color: it.enabled ? 'var(--accent)' : 'var(--text-muted)',
              }"
            >{{ it.enabled ? $t('manage.disable') : $t('manage.enable') }}</button>
            <button @click="removePlugin(it._key!)"
              class="text-[10px] px-1.5 py-0.5 rounded hover:bg-[var(--bg-hover)] transition-colors shrink-0 opacity-0 group-hover:opacity-100"
              :style="{ color: 'var(--coral)' }"
              :title="$t('manage.deletePlugin')"
            >✕</button>
          </template>
          <button v-if="it.path && activeTab !== 'skills' && activeTab !== 'plugins'" @click="openEditor(it.path)" class="text-[10px] px-1.5 py-0.5 rounded hover:bg-[var(--bg-hover)] opacity-0 group-hover:opacity-100 transition-opacity shrink-0" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.edit') }}</button>
          <button v-if="it.path && activeTab === 'skills'" @click="openEditor(it.path + '/SKILL.md')" class="text-[10px] px-1.5 py-0.5 rounded hover:bg-[var(--bg-hover)] opacity-0 group-hover:opacity-100 transition-opacity shrink-0" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.view') }}</button>
        </div>
        <div v-if="items.length === 0 && activeTab !== 'plugins'" class="text-xs py-4 text-center" :style="{ color: 'var(--text-muted)' }">{{ $t('manage.noData') }}</div>
        <!-- Plugins 底部：添加按钮 -->
        <div v-if="activeTab === 'plugins'" class="pt-2">
          <button @click="addPlugin" class="w-full text-xs px-3 py-1.5 rounded border border-dashed transition-colors hover:bg-[var(--bg-hover)]" :style="{ color: 'var(--text-muted)', borderColor: 'var(--border-dim)' }">{{ $t('manage.addPlugin') }}</button>
        </div>
      </div>
    </template>
  </ModalShell>
</template>
