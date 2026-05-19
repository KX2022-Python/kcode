# Kcode TS TUI Migration

Updated: 2026-05-19 JST

## Source Roles

- Authoritative source: `/home/ubuntu/project/kcode`.
- Runtime validation target: `/usr/local/bin/kcode`.
- Legacy audit source: `/home/ubuntu/tools/kcode`, read-only until remaining value is absorbed or cleanup is explicitly approved.
- Local runtime state: `~/.kcode`; never commit sessions, memory, env files, caches, or generated bundles.
- Reference sources: `/home/ubuntu/tools/claude-code-haha` and `/home/ubuntu/tools/claw-code-parity`, both refreshed with `git fetch origin main --prune` on 2026-05-19 without overwriting local changes.

## Runtime Boundary

- Rust engine owns runtime, tools, sessions, providers, MCP, memory, bridge, permission policy, hooks, persistence, headless, and print paths.
- TS/React/Ink TUI owns interaction, layout, input handling, message rendering, command palette, permission dialog, and status/control presentation.
- `kcode` with no arguments defaults to the TS TUI when its bundle is installed.
- `kcode --headless ...` and `/usr/local/bin/kcode-engine ...` run the Rust engine directly.
- `KCODE_TUI=rust` keeps the ratatui fallback available while migration continues.
- The TS TUI input path keeps idle typing in terminal canonical mode for IME compatibility and scrollback, while backend turns route through JSONL.

## Implemented JSONL Protocol

Current typed protocol files:

- `tui/src/protocol/events.ts`
- `tui/src/protocol/types.ts`
- `rust/crates/kcode-cli/src/main_parts/ts_tui_engine_types.rs`
- `rust/crates/kcode-cli/src/main_parts/ts_tui_engine_json.rs`

Events are one JSON object per line. Implemented event families are:

- `bootstrap`, `commands`, `status`, `progress`, `done`, `cancel`, `error`
- `user_submit`, `assistant_delta`, `thinking`
- `tool` call/result events rendered from Rust session messages
- `permission_request` and `permission` decision status events

The TS runtime controller sends `user_submit`, `slash`, `command_list`, `status`, `cancel`, `permission_response`, and `exit` back to the Rust engine.

## Command Surface

- The command palette is driven by Rust `CommandDescriptor` metadata, not a local TS command list.
- TS filters only the backend-provided list for search; backend determines visibility, source, kind, aliases, resume support, safety flags, and wiring.
- Compatibility placeholders and non-wired local settings commands are hidden from the local CLI command surface.
- Informational commands remain visible with `wiring: informational`; executable commands are tagged `wiring: executable`.
- Skill/process command discovery remains Rust-side and is intentionally not promoted into the TS palette unless backend metadata exposes it.

## Control State

- Status/control panels are derived from Rust `json_status` only.
- Model, permission mode, MCP availability, memory counters, Agent tool availability, and Goal status are backend-derived.
- Goal currently reports `available: false` and `status: not-reported` because this Rust runtime has no goal subsystem wired into the command registry yet. The UI displays that state rather than inventing a local goal.

## Permission Flow

- Runtime authorization remains inside Rust `ConversationRuntime` and `PermissionPolicy`.
- When Rust requires approval, the JSONL permission prompter emits `permission_request` with active mode, required mode, tool name, input, and reason.
- TS renders the dialog and sends `permission_response` with `allow_once`, `allow_turn`, `deny_once`, or `deny_turn`.
- The Rust prompter blocks tool execution until that response arrives, then returns allow/deny to the runtime. This is not a frontend-only preview.

## Legacy Audit Notes

`tools/kcode` still contains useful unsynced product input:

- `issues-to-be-fixed/14-tui-runtime-feedback-and-mouse-ergonomics.md`: diagnosis for non-blocking UI execution, runtime phase feedback, mouse wheel, Ctrl-wheel density, and hover scrollbar.
- Rust ratatui-side experiments under `rust/crates/kcode-cli/src/tui/repl/`. They are not copied into the new default frontend because the migration target is TS/React/Ink.

Runtime/development garbage in `tools/kcode` remains approval-gated for cleanup: `.kcode/sessions/*`, `rust/.kcode/sessions/*`, `rust/.clawd-agents/*`, and `rust/target/`.
