<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useSettingsStore } from "@/stores/settings";
import { connectLLM, removeApprovedScenario, listApprovedScenarios } from "@/lib/tauri-bridge";

const router = useRouter();
const settings = useSettingsStore();

const testResult = ref<string | null>(null);
const testError = ref<string | null>(null);
const isTesting = ref(false);
const approvedList = ref<Array<{ tool_name: string; pattern: string }>>([]);

onMounted(async () => { await loadApproved(); });

async function loadApproved() { try { approvedList.value = await listApprovedScenarios(); } catch { approvedList.value = []; } }

async function handleTest() {
  testResult.value = null; testError.value = null; isTesting.value = true;
  try { testResult.value = await connectLLM(settings.apiKey, settings.baseUrl, settings.model); }
  catch (err) { testError.value = String(err); }
  finally { isTesting.value = false; }
}

async function handleRemoveApproved(tool: string, pattern: string) {
  try { await removeApprovedScenario(tool, pattern); approvedList.value = approvedList.value.filter(a => !(a.tool_name === tool && a.pattern === pattern)); } catch {}
}
</script>

<template>
  <div class="h-full overflow-y-auto">
    <div class="max-w-lg mx-auto p-8">
      <!-- Header -->
      <div class="flex items-center gap-3 mb-8">
        <button @click="router.push('/chat')" class="w-7 h-7 flex items-center justify-center rounded-md transition-colors hover:bg-[var(--bg-hover)]" style="color:var(--text-muted)">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="m15 18-6-6 6-6"/></svg>
        </button>
        <h2 class="text-lg font-semibold tracking-tight" style="color:var(--text-bright)">Settings</h2>
      </div>

      <!-- API Config -->
      <section class="space-y-4 mb-8">
        <div>
          <label class="block text-xs font-medium mb-1.5" style="color:var(--text-muted)">Base URL</label>
          <input v-model="settings.baseUrl" type="text" placeholder="https://api.deepseek.com"
            class="w-full rounded-lg px-3.5 py-2 text-sm outline-none transition-colors"
            style="background:var(--bg-elevated); border:1px solid var(--border-default); color:var(--text-primary); caret-color:var(--accent)" />
        </div>
        <div>
          <label class="block text-xs font-medium mb-1.5" style="color:var(--text-muted)">API Key</label>
          <input v-model="settings.apiKey" type="password" placeholder="sk-…"
            class="w-full rounded-lg px-3.5 py-2 text-sm outline-none transition-colors"
            style="background:var(--bg-elevated); border:1px solid var(--border-default); color:var(--text-primary); caret-color:var(--accent)" />
        </div>
        <div>
          <label class="block text-xs font-medium mb-1.5" style="color:var(--text-muted)">Model</label>
          <input v-model="settings.model" type="text" list="model-suggestions" placeholder="deepseek-v4-pro[1M]"
            class="w-full rounded-lg px-3.5 py-2 text-sm outline-none transition-colors"
            style="background:var(--bg-elevated); border:1px solid var(--border-default); color:var(--text-primary); caret-color:var(--accent)" />
          <datalist id="model-suggestions">
            <option value="deepseek-v4-pro[1M]" />
            <option value="deepseek-v4-flash" />
          </datalist>
        </div>
        <button @click="handleTest" :disabled="isTesting || !settings.apiKey"
          class="w-full py-2.5 rounded-lg text-sm font-medium transition-all duration-150"
          :style="{
            background: isTesting ? 'var(--bg-elevated)' : 'var(--accent)',
            color: isTesting ? 'var(--text-muted)' : '#09090b',
            opacity: (!settings.apiKey) ? 0.3 : 1
          }">
          {{ isTesting ? 'Testing…' : 'Test Connection' }}
        </button>
        <div v-if="testResult" class="p-3 rounded-lg text-xs" style="background:var(--accent-glow); color:var(--accent)">✓ {{ testResult }}</div>
        <div v-if="testError" class="p-3 rounded-lg text-xs" style="background:var(--coral-glow); color:var(--coral); border:1px solid var(--coral); --tw-border-opacity:0.3">✕ {{ testError }}</div>
      </section>

      <!-- Presets -->
      <section class="mb-8 pt-6" style="border-top:1px solid var(--border-dim)">
        <h3 class="text-xs font-semibold mb-3 uppercase tracking-widest" style="color:var(--text-muted)">Quick Presets</h3>
        <div class="space-y-2">
          <button @click="settings.baseUrl='https://api.deepseek.com'; settings.model='deepseek-v4-pro[1M]'"
            class="w-full text-left px-4 py-2.5 rounded-lg text-sm transition-colors hover:bg-[var(--bg-elevated)]" style="border:1px solid var(--border-dim); color:var(--text-secondary)">
            <span style="color:var(--text-primary)">DeepSeek</span>
            <span class="block text-xs mt-0.5" style="color:var(--text-muted)">api.deepseek.com</span>
          </button>
        </div>
      </section>

      <!-- Approved Scenarios -->
      <section class="pt-6" style="border-top:1px solid var(--border-dim)">
        <h3 class="text-xs font-semibold mb-1 uppercase tracking-widest" style="color:var(--text-muted)">Auto-Approved Tools</h3>
        <p class="text-xs mb-3" style="color:var(--text-muted)">Tools you've approved will be auto-allowed in future sessions.</p>
        <div class="space-y-1">
          <div v-for="(item, idx) in approvedList" :key="idx"
            class="flex items-center justify-between px-3 py-2 rounded-lg text-sm"
            style="background:var(--bg-elevated); color:var(--text-secondary)">
            <code class="text-xs font-mono" style="color:var(--accent)">{{ item.tool_name }}</code>
            <button @click="handleRemoveApproved(item.tool_name, item.pattern)" class="w-5 h-5 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-hover)]" style="color:var(--text-muted)">
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6 6 18M6 6l12 12"/></svg>
            </button>
          </div>
          <div v-if="approvedList.length === 0" class="text-xs py-4 text-center" style="color:var(--text-muted)">None yet — tools will appear here after you approve them</div>
        </div>
      </section>
    </div>
  </div>
</template>
