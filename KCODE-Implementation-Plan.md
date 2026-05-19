# Kcode Implementation Plan

Updated: 2026-05-19 JST

## Goal

Make `/home/ubuntu/project/kcode` the only authoritative source tree, install and validate `/usr/local/bin/kcode` from that tree, and migrate the default interactive terminal experience toward a TypeScript/React/Ink frontend while preserving the Rust runtime as the engine boundary.

## Hard Constraints

- Authoritative source: `/home/ubuntu/project/kcode`.
- Installed acceptance target: `/usr/local/bin/kcode`.
- Legacy audit source: `/home/ubuntu/tools/kcode`; do not delete it without explicit approval.
- Never commit secrets, runtime env files, sessions, caches, node modules, generated bundles, or Rust targets.
- Run a secret-pattern scan before `git add`.
- Keep code and product docs synchronized in the same round.
- Validate the installed binary, not only source-tree binaries.

## Phase 0: Baseline And Directory Roles

- Freeze `/home/ubuntu/project/kcode` as the write target.
- Audit `/home/ubuntu/tools/kcode` for unsynced useful work.
- Treat `~/.kcode` as runtime state only.
- Keep cleanup as an approval-gated follow-up after listing space benefit and risk.

## Phase 1: Reference Refresh

- Refresh `cc-haha` and absorb interaction ideas only: `/goal` visibility, agent readability, structured fallback, subagent progress, desktop readability.
- Refresh `claw-code-parity` and absorb engine-quality ideas only: help fallthrough, context-window preflight, orphan module audit, JSON/output contract, dead code cleanup.
- Do not copy Anthropic/OAuth/session bindings from references.

## Phase 3: Minimal TS TUI

- Add a `tui/` TypeScript package using React and Ink.
- Keep the TS package focused on interaction, layout, input, message rendering, permission dialogs, goal visibility, and agent progress readability.
- Keep Rust responsible for runtime, tools, sessions, providers, MCP, memory, bridge behavior, and permission policy.
- Define the JSONL event contract in code before full streaming integration.

## Phase 5: Installed Acceptance

- Build the TS frontend.
- Build the Rust release binary.
- Install:
  - `/usr/local/bin/kcode`
  - `/usr/local/bin/kcode-engine`
  - `/usr/local/lib/kcode/tui/dist/index.js`
- Validate:
  - `kcode doctor`
  - `kcode --help`
  - `kcode -p`
  - default TUI smoke path
  - `/goal`
  - `/status`
  - `/mcp`
  - `/memory`
  - permission dialog path

## Phase 6: Cleanup And Archive

- Produce cleanup candidates only until explicit approval is granted.
- Candidate categories: Rust `target/`, `node_modules`, npm/pnpm cache, old backups, Docker reclaimables, journald history, `.clawd-agents`, stale session/log directories.
- Preserve this plan, migration notes, and dev logs under the repo before any cleanup.
- After approved cleanup, revalidate installed `kcode` and relevant local service health.
