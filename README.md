# Kcode

Private terminal agent CLI being converged from the imported Rust `claw-code` workspace into a smaller, maintainable Kcode baseline.

## Current Focus

- keep the Rust workspace as the only production line
- remove mixed-repository residue from the imported source
- establish the Kcode user-facing baseline before bootstrap and provider work

## Repository Layout

```text
.
├── docs/
│   ├── phase-1-repo-convergence.md
│   ├── phase-2-brand-and-platform-baseline.md
│   └── phase-3-bootstrap-and-setup-baseline.md
├── rust/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── crates/
│       ├── api/
│       ├── commands/
│       ├── plugins/
│       ├── runtime/
│       ├── rusty-claude-cli/
│       ├── telemetry/
│       └── tools/
├── LICENSE
└── README.md
```

## Build

The imported Rust workspace currently builds from `rust/`.

If the machine has a broken global `rustup` setup, use an isolated task-local toolchain first:

```bash
export CARGO_HOME=/home/ubuntu/.cache/kcode-cargo
export RUSTC=/home/ubuntu/.cache/kcode-rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin/rustc
export PATH=/home/ubuntu/.cache/kcode-rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin:$PATH
/home/ubuntu/.cache/kcode-rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin/cargo build --manifest-path rust/Cargo.toml
```

## Current CLI Baseline

- primary binary name: `kcode`
- primary user config home: `~/.kcode`
- primary project config path: `./.kcode`
- primary session path: `./.kcode/sessions`
- primary bootstrap commands: `kcode init`, `kcode doctor`, `kcode config show`
- primary env keys: `KCODE_CONFIG_HOME`, `KCODE_PERMISSION_MODE`, `KCODE_SESSION_DIR`, `KCODE_MODEL`, `KCODE_BASE_URL`, `KCODE_API_KEY`

Legacy `claw/.claw/CLAW_*` inputs are still read for migration safety, but new help text and new writes target the Kcode names.

## Next

Phase 4 can now move on to control-plane and provider/profile work on top of the new bootstrap baseline.
