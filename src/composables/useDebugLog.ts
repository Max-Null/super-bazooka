import { ref } from "vue";

const lines = ref<string[]>([]);
const visible = ref(false);

export function useDebugLog() {
  function add(line: string) {
    lines.value.push(line);
    // Keep last 200 lines max
    if (lines.value.length > 200) {
      lines.value = lines.value.slice(-200);
    }
  }

  function toggle() {
    visible.value = !visible.value;
  }

  function clear() {
    lines.value = [];
  }

  return { lines, visible, add, toggle, clear };
}
