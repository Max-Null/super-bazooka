import { useRouter } from "vue-router";
import { useSessionStore } from "@/stores/session";
import { useChatStore } from "@/stores/chat";
import { useDebugLog } from "@/composables/useDebugLog";
import { useSettingsStore } from "@/stores/settings";

/**
 * Shared "new session" logic — used by both the sidebar button and the
 * AppShell navbar button so they stay in sync without duplicating code.
 */
export function useNewSession() {
  const router = useRouter();
  const sessionStore = useSessionStore();
  const chatStore = useChatStore();
  const debugLog = useDebugLog();
  const settings = useSettingsStore();

  async function handleNew() {
    await sessionStore.createSession(settings.model, undefined, undefined, settings.locale);
    chatStore.clearMessages();
    debugLog.clear();
    router.push("/chat");
  }

  return { handleNew };
}
