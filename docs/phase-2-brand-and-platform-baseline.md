# Phase 2: Brand And Platform Baseline

## Scope

Phase 2 establishes the first user-facing Kcode baseline without removing the legacy compatibility layer that still exists for imported `claw` behavior.

## Accepted Baseline

- Primary CLI name: `kcode`
- Primary user config directory: `~/.kcode`
- Primary project config directory: `./.kcode`
- Primary session directory: `./.kcode/sessions`
- Primary environment variables:
  - `KCODE_CONFIG_HOME`
  - `KCODE_PERMISSION_MODE`
  - `KCODE_SESSION_DIR`

## Compatibility Kept On Purpose

The following legacy inputs still load for migration safety during the bootstrap period:

- `CLAW_CONFIG_HOME`
- `RUSTY_CLAUDE_PERMISSION_MODE`
- `./.claw.json`
- `./.claw/settings.json`
- `./.claw/settings.local.json`
- `./.claw/sessions`
- `CLAUDE.md`
- `./.claw/CLAUDE.md`
- `./.claw/instructions.md`

Legacy compatibility is read-only from the product point of view. New user-facing help and new writes now target the Kcode names and paths.

## User-Facing Changes Landed

- `--help` now renders `kcode` command examples
- `/config`, `/memory`, `/init`, `/plugin` help text now uses Kcode branding
- `init` now scaffolds `.kcode/`, `.kcode.json`, and `KCODE.md`
- session help and saved-session flows now point to `.kcode/sessions`
- version output now reports `Kcode`

## Platform Notes

- Linux and macOS continue to use dot-directories in the user home and project root
- Windows still needs a later pass for install-path and shell-specific instructions
- OAuth/login commands remain present only as legacy carry-over and will be addressed in Phase 3
- The imported GitHub workflow omission from Phase 0 remains deferred to release/closeout review

## Verification

- `cargo build --manifest-path rust/Cargo.toml`
- `cargo test -p rusty-claude-cli --test cli_flags_and_config_defaults --test resume_slash_commands`
- direct smoke check with `target/debug/kcode --help`
