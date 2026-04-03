# Phase 3: Bootstrap And Setup Baseline

## Scope

Phase 3 removes the imported OAuth-first startup assumption from the Kcode CLI entry path and replaces it with an explicit bootstrap/setup baseline.

## Accepted Baseline

- `kcode init` now bootstraps the user config home under `~/.kcode`
- `kcode doctor` diagnoses bootstrap readiness before any model request is attempted
- `kcode config show` exposes discovered config files and the merged effective config surface
- prompt and interactive startup now require explicit bootstrap inputs instead of silently falling back to the imported default upstream endpoint

## User-Facing Behavior Landed

- `init` creates:
  - `~/.kcode/config.toml`
  - `~/.kcode/sessions/`
  - `~/.kcode/logs/`
- `doctor` reports:
  - config file presence
  - model selection
  - base URL readiness
  - API credential readiness
  - session directory writeability
  - legacy `.claw` / `CLAUDE.md` residue
- `config show` now includes:
  - config home
  - session dir
  - effective model
  - discovered and loaded config files
  - merged config sections

## Compatibility Kept On Purpose

- legacy OAuth credentials are still readable as a compatibility fallback
- legacy `CLAW_CONFIG_HOME`, `.claw*`, and `CLAUDE.md` artifacts are still detected and reported
- `login` / `logout` remain present as legacy carry-over, but they are no longer the startup prerequisite for prompt mode

## Technical Notes

- `runtime::ConfigLoader` now discovers `config.toml` from both user and project Kcode config roots
- TOML bootstrap keys such as `permission_mode`, `base_url`, `api_key_env`, and `session_dir` now participate in runtime setup
- permission-mode precedence is resolved per loaded file so mixed alias forms do not break later-source override behavior

## Verification

- `cargo build --manifest-path rust/Cargo.toml`
- `cargo test -p runtime loads_kcode_toml_config_files`
- `cargo test -p rusty-claude-cli --bin kcode doctor_report`
- `cargo test -p rusty-claude-cli --bin kcode parses_login_and_logout_subcommands`
- `cargo test -p rusty-claude-cli --test cli_flags_and_config_defaults --test resume_slash_commands`
