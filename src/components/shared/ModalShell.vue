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
            'w-[28rem]': size === 'sm',
            'w-[34rem]': !size || size === 'md',
            'w-[36rem]': size === 'lg',
            'w-[38rem]': size === 'xl',
          },
        ]"
      >
        <!-- 头部 -->
        <div class="shrink-0 flex items-center justify-between px-5 pt-3 pb-2 border-b modal-shell-header">
          <slot name="header" />
          <button
            @click="emit('close')"
            class="w-6 h-6 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-hover)] shrink-0 ml-2 modal-shell-close"
            :title="$t('modal.close')"
          >&times;</button>
        </div>

        <!-- 内容（可滚动） -->
        <div class="overflow-y-auto flex-1 px-5 py-3 modal-shell-body">
          <slot />
        </div>

        <!-- 底部（固定） -->
        <div v-if="$slots.footer" class="shrink-0 px-5 py-2 border-t modal-shell-footer" :style="{ borderColor: 'var(--border-dim)' }">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
