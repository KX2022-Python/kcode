# Kcode

Private terminal agent CLI being converged from the imported Rust `claw-code` workspace into a smaller, maintainable Kcode baseline.

## Current Focus

- keep the Rust workspace as the only production line
- remove mixed-repository residue from the imported source
- prepare the codebase for Kcode branding, config, bootstrap, and release work

## Repository Layout

```text
.
├── docs/
│   └── phase-1-repo-convergence.md
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

## Next

Phase 2 will handle branding, binary/config renaming, and the first user-facing README cleanup.
