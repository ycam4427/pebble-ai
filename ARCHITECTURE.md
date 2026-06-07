# Architecture — Local AI Computer Assistant

## 1. Principles

1. **The AI proposes; it never acts.** The model's only output is data (`Vec<Action>`).
2. **The Safety Validator is independent and authoritative.** It has no dependency on the AI,
   intent, or planner modules, and is the sole gate to the filesystem.
3. **The gate is enforced by types, not discipline.** `ValidatedPlan` has private fields and a
   single constructor (`safety::validate`); the executor accepts only `&ValidatedPlan`.
4. **Defense in depth.** Deny‑list (protected paths) *and* allow‑list (managed sandbox),
   trash instead of delete, full audit log with undo, type‑level execution lock.
5. **Local only.** SQLite + filesystem + localhost Ollama. No network egress beyond Ollama.

## 2. The pipeline

```
                     ┌──────────────────────────  TRUSTED CORE (Rust)  ──────────────────────────┐
 User ─▶ Chat UI ─▶  │ ai/ollama ─▶ intent ─▶ planner ─▶ ⛨ safety::validate ⛨ ─▶ (pending map) │ ─▶ Confirm UI ─▶ executor
        (React)      │  suggestion   parse     concrete      independent of AI                    │   (Tier 1/2)     trash·undo·db
                     └────────────────────────────────────────────────────────────────────────────┘
```

- **Read‑only (Tier 0)** actions run immediately (after a path check) and return result cards.
- **Mutations (Tier 1/2)** become a `ValidatedPlan`, stored server‑side; the frontend only ever
  holds a *view* and references the plan by id. A validated plan never round‑trips through the
  (untrusted) UI.

## 3. Folder structure

```
src/                         React + TS frontend (UI only — no FS access)
├─ lib/        types.ts (mirror Rust) · ipc.ts (typed invoke) · format.ts
├─ store/      appStore.ts (Zustand)
├─ styles/     app.css (dark design system)
└─ components/ Sidebar · Toast · chat/* · actions/* · files/* · trash/* · history/* · settings/*

src-tauri/src/               Rust trusted core
├─ lib.rs · main.rs · commands.rs · state.rs · fsutil.rs
├─ models/    action.rs (Action/Tier/OpKind/Operation/ValidatedOp/Verdict) · data.rs · message.rs · records.rs
├─ ai/        ollama.rs (HTTP client, catalog, VRAM) · prompt.rs (system prompt + schema)
├─ intent/    parse model JSON → Vec<Action> (lenient, drops junk)
├─ planner/   Action → concrete Operation(s); runs Tier‑0 queries; path expansion
├─ safety/    validator.rs (the engine) · paths.rs (normalize/contain) · rules.rs · mod.rs (ValidatedPlan, sealed)
├─ executor/  runs a ValidatedPlan; writes undo log
├─ trash/     move‑to‑trash · restore · empty · retention cleanup
├─ undo/      reverse logged operations
├─ fsops/     scan.rs (large/dupes/stale/search/storage/analyze/list) · summarize.rs
├─ db/        SQLite data access · schema.sql
├─ memory/    conversation context + remembered locations
├─ plugin/    Plugin trait + registry + roadmap (future)
└─ platform/  windows.rs (protected/managed/known folders) · unix.rs (future) · mod.rs
```

## 4. Database schema (SQLite, bundled)

`conversations`, `messages`, `plans`, `action_log` (with `undo_data_json`), `trash_items`
(restore + `expires_at`), `preferences`, `locations`. See `src-tauri/src/db/schema.sql`.

## 5. Action model & tiers

`Action` (AI‑facing, closed set) → `Operation` (concrete, resolved) → `ValidatedOp` (with tier +
verdict + warnings) inside a sealed `ValidatedPlan`.

| Tier | Kinds | Confirmation |
|------|-------|--------------|
| Auto (0) | read/search/analyze/storage/summarize | none (path‑checked) |
| Confirm (1) | move, rename, organize | approve |
| High risk (2) | delete (→trash), run program | approve; typed `CONFIRM DELETE` for >1000 files |

Escalation: any plan over the bulk threshold (100 ops) is forced to high‑risk.

## 6. Safety Validator — eight checks per operation

1. Canonicalize each path (resolve `..`, symlinks; non‑existent move targets resolve their
   deepest existing ancestor).
2. Reject drive/filesystem roots.
3. **Deny‑list**: reject anything under a protected system root (component‑wise, case‑insensitive
   on Windows — so `C:\Windows` ≠ `C:\WindowsApps`).
4. **Self‑protection**: reject the app's own data directory.
5. **Allow‑list (sandbox)**: mutations must be under a managed root (home by default).
6. Tier assignment by kind, then threshold escalation.
7. Trash enforcement: deletes are executed as trash‑moves; permanent delete is unreachable from a
   user command.
8. Execute is rejected unless explicitly enabled in Settings and the program isn't in a protected
   location.

Output is a `ValidatedPlan` whose fields are private; only `safety::validate` (a child module of
`safety`) can build it, and `executor::execute(&ValidatedPlan, …)` is the only consumer.

## 7. Ollama integration

`reqwest` → `http://localhost:11434`: `/api/chat` (forced JSON output for reliable action
parsing), `/api/tags` (catalog), `/api/ps` (live VRAM), `/api/version`. Tokens/sec is derived from
`eval_count / eval_duration`. The model is selectable and persisted in `preferences`.

## 8. UI

Dark theme. Left sidebar (Chat/Files/Trash/History/Settings + conversation list + Ollama status),
center view per tab, right panel on Chat = **Proposed Actions / Safety / Execution Preview** with
Approve · Reject (and the typed high‑risk gate). On other tabs, a proposed plan appears as a modal.

## 9. Extensibility

`plugin::Plugin` defines the future extension surface (calendar, email, browser automation, coding
assistant, OCR, document indexing, knowledge base, voice, smart home). Plugins may only *propose*
actions — everything still flows through the validator. None are implemented in the MVP.
