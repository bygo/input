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
