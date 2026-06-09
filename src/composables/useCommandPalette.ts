import { ref } from "vue";

/** Lightweight event bus to trigger command palette from anywhere */
const trigger = ref(0);

export function useCommandPaletteBus() {
  function open() {
    trigger.value++;
  }
  return { trigger, open };
}
