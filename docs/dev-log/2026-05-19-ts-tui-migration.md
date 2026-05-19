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

- Replaced the old TS skeleton with a TypeScript/React/Ink frontend under `tui/src/App`, `tui/src/components`, `tui/src/runtime`, `tui/src/state`, and `tui/src/protocol`.
- Added a Rust JSONL engine entrypoint: `kcode --headless tui-engine` and `kcode-engine --headless tui-engine`.
- `tui/src/runtime/RuntimeController.ts` spawns the Rust engine, parses JSONL stdout, and sends JSONL control events over stdin.
- Slash palette entries are now backend-driven through Rust command registry metadata, including safety, source, kind, aliases, resume support, and wiring.
- Permission approval is wired through Rust runtime policy: Rust emits `permission_request`, TS returns `permission_response`, and Rust blocks tool execution until the decision is received.
- No-arg `kcode` launches the installed TS TUI bundle when available; `KCODE_TUI=rust` keeps the ratatui fallback.

## Verification Notes

- `npm --prefix tui run check` passed.
- `npm --prefix tui run build` produced `tui/dist/index.js`.
- `cargo check --workspace` passed.
- `cargo test --workspace` passed.
- JSONL engine smoke validated command filtering, backend status controls, and active permission denial flow using a local OpenAI-compatible fake server.

## Input Fix Follow-up

- Idle TS TUI input keeps terminal canonical mode so IME composition and UTF-8 text stay under terminal handling.
- Running backend turns can show permission prompts and status from backend events rather than local fake state.
- ESC/Ctrl+C cancellation remains a frontend control path; hard cancellation of an in-flight Rust model request is still limited by the synchronous engine loop.
