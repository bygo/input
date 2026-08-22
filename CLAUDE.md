# 核心原则

- **语言**：必须使用中文输出。
- **风格**：极致优雅简洁。只实现核心必要逻辑
- **禁令**：禁止添加任何视觉特效（动效/过渡）、禁止添加任何未明确要求的富余逻辑。

# 编码规范 (最高优先级)

- **拒绝封装**：函数必须内联。减少依赖链路，禁止提取不必要的辅助函数（helper functions），用最直接的方法就地调用。
- **扁平结构**：严控文件数量。优先单文件实现，禁止过度拆分目录层级。

# Conduct Rules (Stay Clean)

- **不留垃圾**：任务结束前清理所有临时文件、调试代码和注释掉的旧代码。仓库里不留过程产物。
- **改动聚焦**：只改与任务直接相关的代码，禁止顺手重构、格式化或调整无关内容。
- **就地修改**：优先编辑已有文件，不无故新建文件；新建前先确认确无可复用之处。

<!-- AIPEEK:START -->
# aipeek — Runtime Browser Inspector

IMPORTANT: Before debugging any UI issue, visual bug, or runtime error, ALWAYS fetch the live app state first. Do NOT guess — look at the actual browser state.

## Read state — cheapest first

```bash
curl http://localhost:1420/__aipeek/screen   # state-machine projection {view, modal, focus, knobs} — START HERE
curl http://localhost:1420/__aipeek/ui                  # React component tree — deep-dive when /screen isn't enough
curl 'http://localhost:1420/__aipeek/dom?scope=ChatInput' # semantic DOM scoped to a component — UI as text, src locations
curl http://localhost:1420/__aipeek          # high-density summary (ok sections → 1 line, issues → expanded)
curl http://localhost:1420/__aipeek?full     # full dump: UI tree + console + network + errors + state
curl http://localhost:1420/__aipeek/check    # pass/fail health check — use after code changes
curl http://localhost:1420/__aipeek/console  # console logs (errors, warnings, info)
curl http://localhost:1420/__aipeek/network  # fetch/XHR requests with status and timing
curl http://localhost:1420/__aipeek/errors   # uncaught errors and unhandled rejections
curl http://localhost:1420/__aipeek/state    # registered store snapshots
curl 'http://localhost:1420/__aipeek/query?sel=[role=menuitem]' # this selector right now: count + each element's text/visible/attrs
```

`/query` is the read-side twin of click/fill's `sel=` — assert on a specific element
(how many match, its text, `data-state`, `aria-*`/`data-*`, value, disabled) without `/eval`.

`/screen` projects the whole UI to a few state variables — start there, not `/ui`. Append
`?full` for untruncated output. Append `/{index}` for a specific item's detail.

To inspect or edit a component, work top-down — the full DOM is huge, a scoped view is
accurate: `/screen` or `/ui` to find the component, then `/dom?scope=<Name>` (matches the
source path) or `/dom?sel=<css>` for just that subtree. Each line carries its source
location (`@File.tsx:line`), so the DOM view tells you exactly where to edit.

## Drive the page (acts on the currently-open tab — no separate browser)

```bash
curl 'http://localhost:1420/__aipeek/click?text=New'            # click by visible text (or sel= for CSS)
curl 'http://localhost:1420/__aipeek/fill?sel=textarea&value=hi' # set an input/textarea/select value
curl 'http://localhost:1420/__aipeek/press?key=Enter'           # key on focused element (e.g. Control+a)
curl 'http://localhost:1420/__aipeek/wait?text=Done&timeout=8000' # poll until text/sel appears (add gone=1 to wait until it disappears)
curl 'http://localhost:1420/__aipeek/screenshot?out=shot.png'   # DOM→PNG into .aipeek/ (html-to-image; lossy)
```

`click`/`fill`/`press` settle the DOM and append the resulting UI tree (`--- ui after ---`)
to the response — no follow-up read needed. On a miss, the response lists the reachable
clickable elements so you can re-target. URL-encode any `sel=` with non-ASCII or quotes:
`curl -G http://localhost:1420/__aipeek/click --data-urlencode 'sel=button[title="知识库"]'`.

**Chain — a whole interaction in one round-trip.** POST a JSON array; runs in sequence,
each step settles before the next, stops on first failure:

```bash
curl -X POST http://localhost:1420/__aipeek/chain -d '[
  {"type":"click","sel":"button[title=\"知识库\"]"},
  {"type":"wait","text":"Done"},
  {"type":"fill","sel":"textarea","value":"hi"},
  {"type":"press","key":"Enter"}
]'
```

**Escape hatch.** `curl 'http://localhost:1420/__aipeek/eval?code=...'` (or POST the code as the body) runs arbitrary
JS in the page and returns the result — for what the typed endpoints can't do (install listeners,
read closures, probe event flow). For count/text/state/attr assertions reach for `/query` first.

aipeek auto-detects errors after HMR and prints them to the terminal — watch for `[aipeek]` messages.
<!-- AIPEEK:END -->
