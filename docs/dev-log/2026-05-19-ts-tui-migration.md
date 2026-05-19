# 2026-05-19 TS TUI Migration Dev Log

## Scope

- Authoritative source is `/home/ubuntu/project/kcode`.
- `/usr/local/bin/kcode` is the runtime acceptance target.
- `/home/ubuntu/tools/kcode` was audited as a legacy source but not modified or deleted.

## Audit Notes

- `tools/kcode` contains legacy ratatui experiments and `issues-to-be-fixed/14-tui-runtime-feedback-and-mouse-ergonomics.md`.
- Useful behavior from that audit was captured as TS TUI product input: runtime feedback, readable progress, permission affordance, mouse/density ergonomics.
- Runtime garbage found there includes `.kcode/sessions`, `rust/.kcode/sessions`, `rust/.clawd-agents`, and `rust/target`; cleanup remains approval-gated.

## Reference Refresh

- `cc-haha` was refreshed with `git fetch origin main --prune`.
- `claw-code-parity` was refreshed with `git fetch origin main --prune`.
- Local dirty worktrees in those reference repos were not overwritten.
- Ideas absorbed: goal readability, agent progress grouping, structured command fallback, help fallthrough, strict JSON/output contracts.

## Implementation Notes

- Added `tui/` as a TypeScript/React/Ink frontend package.
- Added typed JSONL protocol definitions in `tui/src/protocol.ts`.
- Added a Rust launcher path so no-arg `kcode` starts the installed TS TUI when available.
- Added `kcode --headless` and installed `kcode-engine` as Rust engine entry points.
- Kept `KCODE_TUI=rust` as a temporary ratatui fallback.

## Verification Notes

- `npm --prefix tui run check` passed.
- `npm --prefix tui run build` produced `tui/dist/index.js`.
- `cargo check -p kcode-cli` passed.
- Installed-binary validation is tracked separately in the final delivery notes for this round.

## Input Fix Follow-up

- Idle TS TUI input now keeps terminal canonical mode so IME composition, non-Chinese text, and scrollback behavior stay under the terminal.
- Running engine calls can be cancelled from the frontend; ESC/Ctrl+C requests cancellation and the child process receives interrupt/terminate/kill fallback signals.
- Regression coverage now tracks IME text, mouse-wheel scrollback, and ESC cancellation as required TUI behavior.
