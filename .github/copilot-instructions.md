# Copilot instructions for KiKi (quick reference)

This file gives actionable, repository-specific guidance for AI coding agents working on KiKi.

- **Big picture:** KiKi is a small Rust CLI that manages a system `sing-box` installation. Key responsibilities:
  - parse `ss://` Shadowsocks links (`src/commands/set.rs`) and update `/etc/sing-box/config.json`.
  - validate environment and config using the `sing-box` binary (`src/commands/check.rs`).
  - control the systemd `sing-box` service via `systemctl` (`src/commands/service.rs`).

- **Entry points:**
  - CLI wiring: `src/main.rs` — subcommands map to modules under `src/commands/`.
  - Commands: `set`, `check`, `start`, `stop`, `restart`.

- **Important files to reference:**
  - Cargo manifest: `Cargo.toml` (uses `clap`, `serde`, `serde_json`, `base64`).
  - CLI: `src/main.rs`.
  - Command implementations: `src/commands/set.rs`, `src/commands/check.rs`, `src/commands/service.rs`.
  - Installer: `kiki-install.sh` and README usage examples: `README.md`.
  - Managed config: `/etc/sing-box/config.json` (updated in-place by `set`).

- **Parsing & config update pattern (`set.rs`):**
  - Input: `ss://` link. The code first strips `ss://` and `#tag`, then normalizes base64 padding (append `=` until length % 4 == 0).
  - Decoding strategies:
    - Try full-base64 decode of the whole part.
    - If that fails, split around the last `@` and try decoding the left side (user info) separately.
  - After decode, `rfind('@')` is used to split `method:pass@host:port`. Port parsing uses `parse::<u16>()`.
  - The config update uses `serde_json::Value` to locate the `outbounds` array and find the outbound where `type == "shadowsocks"`, then mutates `method`, `password`, `server`, `server_port` and writes back with `serde_json::to_string_pretty()`.
  - Errors are propagated as `Result<..., Box<dyn Error>>` and many failures print to stderr in `main.rs`.

- **Environment & runtime conventions:**
  - The tool assumes it is run with privileges (writes to `/etc` and calls `systemctl`). Typical usage examples in README use `sudo`.
  - The code relies on external binaries: `sing-box` (version check with `sing-box version`) and `systemctl` for service management.
  - `check.rs` runs `sing-box check -c /etc/sing-box/config.json` to validate the config syntax.

- **Build & run (developer flows):**
  - Build: `cargo build` (for development) or `cargo build --release`.
  - Run locally: `cargo run -- <subcommand>` — but note many commands expect root access and system binaries; use `sudo` when testing features that touch `/etc` or `systemctl`.
  - Install/test flow (as in README): run the `kiki-install.sh` installer which configures `sing-box` and installs the CLI.

- **Patterns & gotchas for code edits:**
  - Keep CLI semantics in `src/main.rs` consistent — add subcommands there when adding features.
  - Preserve the base64 padding logic in `set.rs` when changing parsing — it's intentional to tolerate missing padding.
  - `set.rs` uses `rsplitn(2, ':')` for address parsing to be tolerant of IPv6; keep that approach if extending address parsing.
  - Mutating `serde_json::Value` in-place is the current approach; prefer minimal changes to keep file shape stable.
  - Service control is delegated to `systemctl` — do not replace with custom process managers unless adjusting all callers.

- **Examples you can use in patches/tests:**
  - Parse example: `sudo kiki set "ss://YWVzLTI1Ni1jZmI6cGFzc0BleGFtcGxlLmNvbTo4MDgw"` (see `README.md`).
  - Validation call (used by `check`): `sing-box check -c /etc/sing-box/config.json` — inspect stderr for validation details.

- **When making fixes / PRs:**
  - Test logic paths that decode both fully-encoded and partially-encoded `ss://` strings.
  - If adding logging, follow the existing pattern (simple println/eprintln, emoji UI) to keep CLI UX consistent.
  - Avoid hardcoding alternate config paths unless adding a CLI flag; existing code assumes `/etc/sing-box/config.json`.

If anything here is missing or unclear, tell me which area you want expanded (parsing edge-cases, service integration, or build/test commands) and I will update this file.
