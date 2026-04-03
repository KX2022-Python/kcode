# Phase 1 Repo Convergence

Updated: 2026-04-03 JST

## Goal

Turn the imported mixed repository into the Kcode Rust-first baseline described in `/home/ubuntu/memos/KCODE.md`.

This phase is about repository shape, not feature expansion.

## Current State

Imported baseline originally contained:

- `rust/`
- `src/`
- `tests/`
- `assets/`
- `PARITY.md`
- legacy `README.md`
- `.claude*`
- `.github/FUNDING.yml`

Rust workspace compiles when built with an isolated task-local toolchain:

```bash
export CARGO_HOME=/home/ubuntu/.cache/kcode-cargo
export RUSTC=/home/ubuntu/.cache/kcode-rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin/rustc
export PATH=/home/ubuntu/.cache/kcode-rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin:$PATH
/home/ubuntu/.cache/kcode-rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin/cargo build --manifest-path /home/ubuntu/kcode/rust/Cargo.toml
```

## Keep / Drop

Keep for Kcode baseline:

- `rust/Cargo.toml`
- `rust/Cargo.lock`
- `rust/crates/api`
- `rust/crates/commands`
- `rust/crates/plugins`
- `rust/crates/runtime`
- `rust/crates/tools`
- CLI crate, renamed from `rusty-claude-cli` to a Kcode-aligned name in a later step
- top-level minimal `README.md`
- top-level `LICENSE`
- top-level `docs/`
- top-level minimal helper scripts if needed

Drop or isolate from the Kcode production line:

- `src/`
- `tests/`
- `assets/`
- `PARITY.md`
- `.claude/`
- `.claude.json`
- `CLAUDE.md`
- `rust/.claude/`
- `rust/.omc/`
- `rust/.sandbox-home/`
- `rust/.clawd-todos.json`
- `rust/crates/compat-harness`

Reshape, not blindly delete:

- `rust/crates/telemetry`
  Current role is in-process tracing and request profiling.
  Kcode should not ship Anthropic-bound defaults or hidden telemetry paths, but this crate can be retained temporarily as a local observability shell and renamed/reduced later.

## Phase 1 Decisions

1. The repo remains rooted at `/home/ubuntu/kcode`.
2. The Rust workspace remains under `rust/` for now.
3. Phase 1 does not yet move `rust/` to the repo root.
4. We first remove mixed-language and branding noise, then tackle Kcode renaming in Phase 2.
5. `compat-harness` is not part of the Kcode production baseline and should be removed from the default workspace.
6. `telemetry` should stay buildable for now, but later lose Anthropic-specific defaults and any external reporting semantics.

## Executed Cuts

Completed in this phase:

1. Removed top-level Python-porting and marketing content.
2. Removed `.claude*` and Rust-side OmX/sandbox residue.
3. Removed `rust/crates/compat-harness`.
4. Detached the CLI crate from `compat-harness`.
5. Replaced the repository `README.md` with a minimal Kcode-facing baseline README.
6. Added a top-level `LICENSE`.
7. Switched the Rust workspace from wildcard crate discovery to an explicit member list.

## Known Deviations

- The private GitHub mirror omits `.github/workflows/rust-ci.yml` because the current HTTPS token lacks `workflow` scope.
- This is explicitly deferred to final wrap-up and does not block Phase 1.

## Exit Criteria

Phase 1 is complete when:

- the repository no longer presents Python porting content as the main surface
- the Kcode mainline is clearly identifiable as a Rust-first workspace
- the minimal Rust workspace still builds
- the repo shape is ready for Phase 2 branding and directory standardization

Current result:

- achieved with a successful `cargo build` from `rust/` using the task-local toolchain path above
