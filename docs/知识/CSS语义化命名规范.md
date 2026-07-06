# CSS 语义化命名规范

> 可复用的前端 CSS 规范。

## 精简版（直接复制到 CLAUDE.md）

```markdown
## CSS 命名硬约束

DOM 元素必须有语义 class 名，禁止裸 Tailwind 长链：
- class 含 3+ Tailwind token → 提取为语义 class
- inline style 含 3+ 规则 → 收进 class
- BEM 命名：`组件-区块--状态`，名从 HTML 注释来
- 颜色只用 `var(--xxx)`，禁止硬编码色值
- 出现 3+ 次的相同组合 → 提取为全局通用 class

审查命令：`grep -rn 'class="[^"]{40,}"' src/ --include="*.vue"`
```

---

## 核心原则

**DOM 元素必须有语义 class 名。** 裸 Tailwind 长链和 inline style 在代码审查时不可读，沟通时无法定位，重构时无法搜索。

```html
<!-- ❌ 禁止：你说"那个 w-1.5 shrink-0 cursor-col-resize 的 div"我听不懂 -->
<div class="w-1.5 shrink-0 cursor-col-resize hover:bg-[var(--accent)]/30 transition-colors select-none"
     style="background: var(--accent-dim)" />

<!-- ✅ 正确：你说"拖拽把手"我立刻定位 -->
<div class="panel-drag-handle" />
```

## 规则

### 1. 三 token 阈值

任何 `class="..."` 含 **3 个或以上** Tailwind/utility token → 提取为语义 class。

```
❌ class="flex items-center gap-2 px-3 py-2"
✅ class="toolbar-group"
```

### 2. inline style 同等待遇

`style="..."` 含 **3 条或以上** CSS 规则 → 收进 class。

```
❌ style="background: var(--bg-surface); border-bottom: 1px solid var(--border-dim); color: var(--text-muted)"
✅ class="sticky-question-bar"
```

### 3. BEM 风格命名

```
组件前缀-区块--状态
```

| 命名 | 含义 |
|------|------|
| `sb-header` | Shell → Header |
| `panel-drag-handle--active` | Panel → 拖拽把手 → 激活态 |
| `msg-row--user` | 消息行 → 用户消息变体 |
| `file-panel-tab--open` | 文件面板 → 抽屉标签 → 展开态 |

命名来源：**HTML 注释里描述的元素名。** 如果在注释里叫"拖拽把手"，class 就叫 `xxx-drag-handle`。

### 4. 颜色只用 CSS 变量

禁止硬编码色值（`#06d6a0`、`rgba(255,255,255,0.03)` 等），统一用 `var(--xxx)`。

### 5. 通用样式提取

出现 3 次以上的相同 class 组合 → 提取为全局通用 class。

```
❌ 四处重复：
  class="px-3 py-1.5 rounded-md text-xs font-medium transition-colors hover:bg-[var(--bg-hover)]"

✅ 提取到 main.css：
  .btn-ghost { ... }
  四处改为：class="btn-ghost"
```

## 审查命令

一行扫描全项目：

```bash
# 找出所有 class 链超过 40 字符的文件（≈ 4+ Tailwind token）
grep -rn 'class="[^"]{40,}"' src/ --include="*.vue"

# 找出 inline style 过多的文件
grep -c 'style="' src/**/*.vue | sort -t: -k2 -rn | head -20
```

## 迁移流程

1. **扫描** — 运行审查命令，列出违例文件
2. **提取通用类** — 出现 3+ 次的相同组合先提取到全局 CSS
3. **逐个组件收编** — 每个组件内部的长 class → 语义 class + scoped CSS
4. **inline style 归位** — 3 条以上的移到 class，保留单条动态样式（如条件颜色）
5. **删除死代码** — 用 `git grep` 确认旧 class 名无引用后删除

## 例外

以下可以保留 Tailwind/inline：

- **动态条件样式**：`:style="{ color: visible ? 'var(--accent)' : 'var(--text-muted)' }"` —— 条件分支无法用纯 CSS 表达
- **单属性布局微调**：`class="ml-2"`、`class="shrink-0"` —— 2 个以下 token 可以保留
- **第三方组件覆盖**：修改第三方组件样式时优先用 `:deep()` + CSS，其次用单 token 覆盖
