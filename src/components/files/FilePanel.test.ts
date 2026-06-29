import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createI18n } from "vue-i18n";

// ── Mock @tauri-apps/plugin-shell ──
vi.mock("@tauri-apps/plugin-shell", () => ({
  open: () => Promise.resolve(),
  Command: { create: () => ({ execute: () => Promise.resolve() }) },
}));

// ── Mock tauri-bridge ──
const { mockListDir, mockReadFileContent, mockGetWorkspaceRoot } = vi.hoisted(() => ({
  mockListDir: vi.fn().mockResolvedValue([]),
  mockReadFileContent: vi.fn().mockResolvedValue("file content"),
  mockGetWorkspaceRoot: vi.fn().mockResolvedValue("C:\\project"),
}));

vi.mock("@/lib/tauri-bridge", () => ({
  listDir: mockListDir,
  readFileContent: mockReadFileContent,
  getWorkspaceRoot: mockGetWorkspaceRoot,
}));

// ── Mock translateError ──
vi.mock("@/lib/utils", () => ({
  translateError: (e: unknown) => ({ key: "error", params: { message: String(e) } }),
}));

// ── i18n ──
const i18n = createI18n({
  legacy: false,
  locale: "en",
  messages: {
    en: {
      file: {
        title: "Files",
        backToRoot: "Back to Root",
        openPreview: "Open Preview",
        closePreview: "Close Preview",
        refresh: "Refresh",
      },
    },
  },
});

import FilePanel from "./FilePanel.vue";

// 简易 stub
function stub(name: string, template = "<div></div>") {
  return { name, template, props: {} as Record<string, unknown> };
}

function mountPanel() {
  return mount(FilePanel, {
    global: {
      plugins: [i18n],
      stubs: {
        ErrorBoundary: stub("ErrorBoundary", "<div><slot /></div>"),
        FileTree: stub("FileTree"),
        FilePreview: stub("FilePreview"),
        FilePreviewModal: stub("FilePreviewModal"),
      },
    },
  });
}

describe("FilePanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ── openFile：路径存储 ──

  it("openFile stores filename and full path for root-level file", async () => {
    const wrapper = mountPanel();
    await wrapper.vm.$nextTick();

    await wrapper.vm.openFile({ name: "README.md", path: "C:\\project\\README.md", is_dir: false, size: 1024 });

    expect(wrapper.vm.selectedFile).toBe("README.md");
    expect(wrapper.vm.selectedFilePath).toBe("C:\\project\\README.md");
    expect(mockReadFileContent).toHaveBeenCalledWith("C:\\project\\README.md");
  });

  it("openFile stores full path including subdirectories for nested file", async () => {
    const wrapper = mountPanel();
    await wrapper.vm.$nextTick();

    await wrapper.vm.openFile({
      name: "ChatPanel.vue",
      path: "C:\\project\\src\\components\\ChatPanel.vue",
      is_dir: false,
      size: 5000,
    });

    expect(wrapper.vm.selectedFile).toBe("ChatPanel.vue");
    expect(wrapper.vm.selectedFilePath).toBe("C:\\project\\src\\components\\ChatPanel.vue");
  });

  it("openFile navigates to directory instead of previewing", async () => {
    const wrapper = mountPanel();
    await wrapper.vm.$nextTick();

    await wrapper.vm.openFile({ name: "src", path: "C:\\project\\src", is_dir: true, size: 0 });

    // 目录不设 selectedFile
    expect(wrapper.vm.selectedFile).toBeNull();
    expect(wrapper.vm.selectedFilePath).toBe("");
    expect(mockReadFileContent).not.toHaveBeenCalled();
    // 导航到目录
    expect(mockListDir).toHaveBeenCalledWith("C:\\project\\src");
  });

  it("openFile shows error message when readFileContent fails", async () => {
    mockReadFileContent.mockRejectedValueOnce("Permission denied");
    const wrapper = mountPanel();
    await wrapper.vm.$nextTick();

    await wrapper.vm.openFile({ name: "secret.txt", path: "C:\\project\\secret.txt", is_dir: false, size: 0 });

    // 即使读取失败，路径仍正确存储
    expect(wrapper.vm.selectedFile).toBe("secret.txt");
    expect(wrapper.vm.selectedFilePath).toBe("C:\\project\\secret.txt");
    expect(wrapper.vm.previewContent).not.toBe("file content");
    expect(wrapper.vm.previewContent).not.toBe("");
  });

  // ── openModalPreview：使用完整路径 ──

  it("openModalPreview uses selectedFilePath for modal preview", () => {
    const wrapper = mountPanel();

    // 模拟从子目录选中文件后的状态
    wrapper.vm.selectedFile = "App.vue";
    wrapper.vm.selectedFilePath = "C:\\project\\src\\App.vue";
    wrapper.vm.openModalPreview();

    expect(wrapper.vm.previewFile).toEqual({
      name: "App.vue",
      path: "C:\\project\\src\\App.vue",
    });
  });

  it("openModalPreview preserves deep path through subdirectories", () => {
    const wrapper = mountPanel();

    wrapper.vm.selectedFile = "index.ts";
    wrapper.vm.selectedFilePath = "C:\\project\\src\\composables\\useStreamProcessor.ts";
    wrapper.vm.openModalPreview();

    expect(wrapper.vm.previewFile?.path).toBe("C:\\project\\src\\composables\\useStreamProcessor.ts");
    expect(wrapper.vm.previewFile?.name).toBe("index.ts");
  });

  it("openModalPreview does nothing when selectedFile is null", () => {
    const wrapper = mountPanel();

    wrapper.vm.selectedFile = null;
    wrapper.vm.selectedFilePath = "";
    wrapper.vm.openModalPreview();

    expect(wrapper.vm.previewFile).toBeNull();
  });

  it("openModalPreview does nothing when selectedFilePath is empty", () => {
    const wrapper = mountPanel();

    // 有文件名但无完整路径 —— 防御性保护
    wrapper.vm.selectedFile = "orphan.txt";
    wrapper.vm.selectedFilePath = "";
    wrapper.vm.openModalPreview();

    expect(wrapper.vm.previewFile).toBeNull();
  });

  // ── 关闭预览清理状态 ──

  it("close button clears selectedFile, selectedFilePath, previewContent", async () => {
    const wrapper = mountPanel();
    await wrapper.vm.$nextTick();

    // 展开面板才能渲染 inline preview 区域（collapsed 初始为 true）
    wrapper.vm.collapsed = false;

    // 通过 openFile 设置预览状态
    await wrapper.vm.openFile({ name: "test.ts", path: "C:\\project\\test.ts", is_dir: false, size: 100 });
    await wrapper.vm.$nextTick();

    expect(wrapper.vm.selectedFile).toBe("test.ts");
    expect(wrapper.vm.selectedFilePath).toBe("C:\\project\\test.ts");
    expect(wrapper.vm.previewContent).toBeTruthy();

    // 点击关闭按钮（模板 inline preview 的 ×）
    const closeBtn = wrapper.find("button[title='Close Preview']");
    expect(closeBtn.exists()).toBe(true);
    await closeBtn.trigger("click");

    expect(wrapper.vm.selectedFile).toBeNull();
    expect(wrapper.vm.selectedFilePath).toBe("");
    expect(wrapper.vm.previewContent).toBe("");
  });

  // ── 面包屑 ──

  it("pathSegments splits root path into clickable segments", async () => {
    const wrapper = mountPanel();
    await wrapper.vm.$nextTick();

    wrapper.vm.rootPath = "C:\\project\\src\\components";
    await wrapper.vm.$nextTick();

    const segments = wrapper.vm.pathSegments;
    expect(segments).toHaveLength(4);
    expect(segments[0]).toEqual({ label: "C:", fullPath: "C:\\" });
    expect(segments[1]).toEqual({ label: "project", fullPath: "C:\\project" });
    expect(segments[2]).toEqual({ label: "src", fullPath: "C:\\project\\src" });
    expect(segments[3]).toEqual({ label: "components", fullPath: "C:\\project\\src\\components" });
  });
});
