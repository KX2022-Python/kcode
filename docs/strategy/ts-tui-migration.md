# Kcode TS TUI Migration

Updated: 2026-05-19 JST

## Source Roles

- Authoritative source: `/home/ubuntu/project/kcode`.
- Runtime validation target: `/usr/local/bin/kcode`.
- Legacy audit source: `/home/ubuntu/tools/kcode`, read-only until remaining value is absorbed or cleanup is explicitly approved.
- Local runtime state: `~/.kcode`; never commit sessions, memory, env files, caches, or generated bundles.
- Reference sources: `/home/ubuntu/tools/claude-code-haha` and `/home/ubuntu/tools/claw-code-parity`, both refreshed with `git fetch origin main --prune` on 2026-05-19 without overwriting local changes.

## Current Audit

`tools/kcode` still contains these useful unsynced items:

- `issues-to-be-fixed/14-tui-runtime-feedback-and-mouse-ergonomics.md`: good diagnosis for non-blocking UI execution, runtime phase feedback, mouse wheel, Ctrl-wheel density, and hover scrollbar.
- `rust/crates/kcode-cli/src/tui/repl/mouse.rs` and `runtime_render.rs`: Rust ratatui-side interaction experiments. They are not copied into the new default frontend because the migration target is TS/React/Ink, but the behavior is captured as product input.
- Differences in Rust TUI files (`messages.rs`, `mod.rs`, `runtime_loop.rs`, `state.rs`) are legacy ratatui work. They should not be expanded further except as temporary fallback maintenance.

Runtime/development garbage in `tools/kcode`:

- `.kcode/sessions/*`, `rust/.kcode/sessions/*`, `rust/.clawd-agents/*`, and `rust/target/`.
- These are excluded from sync and require explicit cleanup approval before deletion.

## Reference Refresh Findings

cc-haha latest `origin/main` after fetch: `55d4c80 Center v0.2.8 notes on /goal and Agent readability`.

Kcode should absorb the interaction ideas, not its Anthropic/OAuth bindings:

- `/goal` needs persistent top-level visibility and direct command affordance.
- Agent progress should render as readable grouped status rather than raw JSON.
- Structured fallback should show clear system/error messages when a command cannot be handled.
- Desktop readability maps to tighter headers, concise status lines, and message grouping.

claw-code-parity latest `origin/main` after fetch: `ebef748 docs: add subcommand help fallthrough pinpoint`.

Kcode should absorb:

- Help fallthrough must route unknown or partially supported commands to actionable help.
- Context-window preflight belongs in Rust engine before model calls.
- Orphan module audit and dead code cleanup stay as follow-up quality gates.
- JSON/output contracts should remain strict and testable across the Rust engine and TS TUI boundary.

## Runtime Boundary

- Rust engine owns runtime, tools, sessions, providers, MCP, memory, bridge, permission policy, and persistence.
- TS/React/Ink TUI owns interaction, layout, input handling, message rendering, permission dialogs, `/goal` readability, and agent progress readability.
- `kcode` with no arguments defaults to the TS TUI when its bundle is installed.
- `kcode --headless ...` and `/usr/local/bin/kcode-engine ...` run the Rust engine directly.
- `KCODE_TUI=rust` keeps the ratatui fallback available while migration continues.
- The TS TUI input path keeps idle typing in terminal canonical mode for IME compatibility and scrollback, while running engine calls expose ESC/Ctrl+C cancellation.

## JSONL / stdio Protocol Draft

Every event is one JSON object per line with a `type` field:

- `session`: session start/resume/complete plus `sessionId`.
- `assistant_text`: assistant text delta or final text.
- `thinking`: visible thinking/progress text when available.
- `tool_call`: `id`, `name`, and structured `input`.
- `tool_result`: `id`, `name`, `output`, and `isError`.
- `permission_request`: `id`, `toolName`, `inputSummary`, `requiredMode`.
- `goal_state`: `none`, `active`, or `complete` plus optional `objective`.
- `agent_progress`: `agentId`, label, status, and detail.
- `error`: message plus optional recoverability.

The current Phase 3 skeleton shells out to the Rust engine and keeps this protocol typed in `tui/src/protocol.ts`; full streaming stdio wiring is the next implementation step.
