<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";

const props = defineProps<{
  open: boolean;
  /** 垂直位置：center 居中，top 靠上 */
  position?: "center" | "top";
  /** 宽度预设 */
  size?: "sm" | "md" | "lg" | "xl";
}>();
const emit = defineEmits<{ close: [] }>();

// 顶部定位时留出底部空间，防止超出屏幕
const maxHClass = computed(() =>
  props.position === "top" ? "max-h-[70vh]" : "max-h-[85vh]"
);

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}
onMounted(() => document.addEventListener("keydown", onKeydown));
onUnmounted(() => document.removeEventListener("keydown", onKeydown));
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex"
      :class="position === 'top' ? 'items-start justify-center pt-[14vh]' : 'items-center justify-center'"
      style="background: rgba(0,0,0,0.3)"
      @click.self="emit('close')"
    >
      <div
        class="modal-shell-panel rounded-xl overflow-hidden shadow-2xl border flex flex-col"
        :class="[
          maxHClass,
          {
            'w-[400px]': size === 'sm',
            'w-[480px]': !size || size === 'md',
            'w-[580px]': size === 'lg',
            'w-[520px]': size === 'xl',
          },
        ]"
      >
        <!-- 头部 -->
        <div class="shrink-0 flex items-center justify-between px-5 h-12 border-b modal-shell-header">
          <slot name="header" />
          <button
            @click="emit('close')"
            class="w-6 h-6 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-hover)] shrink-0 ml-2 modal-shell-close"
            title="关闭"
          >&times;</button>
        </div>

        <!-- 内容 -->
        <div class="overflow-y-auto flex-1 px-5 py-3 modal-shell-body">
          <slot />
        </div>
      </div>
    </div>
  </Teleport>
</template>
