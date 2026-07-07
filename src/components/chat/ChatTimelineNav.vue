<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from "vue";
import type { Message } from "@/stores/chat";

const props = defineProps<{
  messages: Message[];
  scrollContainer: HTMLElement | null;
}>();

const emit = defineEmits<{ scrollTo: [index: number] }>();

const userMessages = computed(() =>
  props.messages.filter(m => m.role === "user")
);

// 当前滚动到的用户消息索引（scroll spy）
const activeIndex = ref(-1);

function updateActive() {
  const c = props.scrollContainer;
  if (!c) { activeIndex.value = -1; return; }
  const userEls = c.querySelectorAll<HTMLElement>('[data-role="user"]');
  if (userEls.length === 0) { activeIndex.value = -1; return; }
  // 最后一个顶部已滚出视口的用户消息（即当前可见的第一条或刚滚过的）
  let best = 0;
  userEls.forEach((el, i) => {
    if (el.offsetTop <= c.scrollTop + 80) best = i;
  });
  activeIndex.value = best;
}

let timer: ReturnType<typeof setTimeout> | null = null;
function scheduleUpdate() {
  if (timer) return;
  timer = setTimeout(() => { timer = null; updateActive(); }, 80);
}

watch(() => props.messages.length, () => nextTick(() => scheduleUpdate()), { immediate: true });
watch(() => props.scrollContainer, (c) => {
  if (c) c.addEventListener("scroll", scheduleUpdate, { passive: true });
}, { immediate: true });
onMounted(() => {
  window.addEventListener("resize", scheduleUpdate);
  nextTick(() => scheduleUpdate());
});
onUnmounted(() => {
  props.scrollContainer?.removeEventListener("scroll", scheduleUpdate);
  window.removeEventListener("resize", scheduleUpdate);
  if (timer) clearTimeout(timer);
});

const hoveredIndex = ref(-1);

function onClick(index: number) {
  emit("scrollTo", index);
}
</script>

<template>
  <div v-if="userMessages.length > 0" class="chat-timeline-nav">
    <div
      v-for="(msg, i) in userMessages"
      :key="i"
      class="chat-timeline-dot"
      :class="{ 'chat-timeline-dot--active': activeIndex === i }"
      @mouseenter="hoveredIndex = i"
      @mouseleave="hoveredIndex = -1"
      @click="onClick(i)"
    >
      <Transition name="tooltip-fade">
        <div v-if="hoveredIndex === i" class="chat-timeline-tooltip">
          {{ msg.content?.slice(0, 80) || "" }}
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.chat-timeline-nav {
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
  width: 24px;
  z-index: 5;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  pointer-events: none;
}

.chat-timeline-dot {
  position: relative;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--border-bright);
  cursor: pointer;
  pointer-events: auto;
  flex-shrink: 0;
  transition: background 150ms, scale 150ms;
}
.chat-timeline-dot:hover {
  background: var(--accent);
  scale: 1.6;
}
.chat-timeline-dot--active {
  background: var(--accent);
  scale: 1.4;
  box-shadow: 0 0 6px var(--accent);
}

.chat-timeline-tooltip {
  position: absolute;
  right: calc(100% + 8px);
  top: 50%;
  translate: 0 -50%;
  max-width: 260px;
  padding: 3px 8px;
  font-size: 11px;
  line-height: 1.4;
  color: var(--text-primary);
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  pointer-events: none;
}

.tooltip-fade-enter-active { transition: opacity 100ms ease-out; }
.tooltip-fade-leave-active { transition: opacity 60ms ease-in; }
.tooltip-fade-enter-from, .tooltip-fade-leave-to { opacity: 0; }
</style>
