# Copilot instructions for KiKi (quick reference)

This file gives actionable, repository-specific guidance for AI coding agents working on KiKi.

- **Big picture:** KiKi is a small Rust CLI that manages a system `sing-box` installation. Key responsibilities:
  - parse `ss://` Shadowsocks, `vmess://` VMess, `trojan://` Trojan, `vless://` VLESS, and `hy2://` Hysteria2 links (`src/commands/set.rs`) and update `/etc/sing-box/config.json`.
  - validate environment and config using the `sing-box` binary (`src/commands/check.rs`).
  - control the systemd `sing-box` service via `systemctl` (`src/commands/service.rs`).

- **Entry points:**
  - CLI wiring: `src/main.rs` — subcommands map to modules under `src/commands/`.
  - Commands: `set`, `check`, `start`, `stop`, `restart`, `logs`.

- **Important files to reference:**
  - Cargo manifest: `Cargo.toml` (uses `clap`, `serde`, `serde_json`, `base64`).
  - CLI: `src/main.rs`.
  - Command implementations: `src/commands/set.rs`, `src/commands/check.rs`, `src/commands/service.rs`, `src/commands/logs.rs`.
  - Installer: `kiki-install.sh` and README usage examples: `README.md`.
  - Managed config: `/etc/sing-box/config.json` (updated in-place by `set`).

- **Parsing & config update pattern (`set.rs`):**
  - **Shadowsocks**: Input `ss://` link, strips prefix and `#tag`, then normalizes base64 padding (append `=` until length % 4 == 0).
    - Decoding strategies: Try full-base64 decode; if fails, split around last `@` and decode left side separately.
    - After decode, `rfind('@')` splits `method:pass@host:port`. Port parsing uses `parse::<u16>()`.
  - **VMess**: Input `vmess://` link (base64-encoded JSON with fields: `id`, `add`, `port`, `scy`, `aid`, `net`, `host`, `path`, `tls`, `sni`, `alpn`, `fp`).
    - Decodes base64 to JSON, extracts UUID, server, port, and optional transport/TLS parameters.
    - Supports transports: `tcp` (default), `ws` (WebSocket with path/host), `h2` (HTTP/2 with path/host).
  - **Trojan**: Input `trojan://password@server:port?...`. Directly parses URI format.
  - **VLESS**: Input `vless://uuid@server:port?flow=...&security=tls&sni=...&host=...&path=...&alpn=...`. Supports flow modes, TLS, and transport options.
  - **Hysteria2**: Input `hysteria2://uuid@server:port?peer=...&insecure=...&obfs=...&obfs-password=...` or `hy2://...`. UUID stored as password field. Obfs defaults to "salamander". Supports peer, insecure, obfs, and obfs-password params. Also supports `sni` and `alpn`.
  - Config updates: Always find outbound with `tag == "proxy"`, clear all fields except tag, then set protocol-specific fields to ensure no stale config remains.
  - Errors propagated as `Result<..., Box<dyn Error>>`; failures print to stderr via `main.rs`.

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
  - Protocol detection in `set.rs`: use `url.starts_with()` to dispatch to protocol-specific handlers (`handle_shadowsocks`, `handle_vmess`, `handle_trojan`, `handle_vless`, `handle_hysteria2`).
  - Preserve base64 padding logic (append `=` until length % 4 == 0) in Shadowsocks and VMess parsing — tolerates missing padding by design.
  - Shadowsocks/Trojan/VLESS/Hysteria2 use `rsplitn(2, ':')` for address parsing to tolerate IPv6; keep that if extending.
  - VMess/VLESS/Trojan transport/TLS construction uses conditional JSON building: omit empty/default fields (e.g., `host`, `path`, `fingerprint`).
  - Always update outbound with `tag == "proxy"` (not by type) to preserve routing consistency and clear old protocol fields.
  - Config updates: Reset object to `{ "tag": tag }` first, then add protocol-specific fields to avoid stale config remnants.
  - Hysteria2: UUID is stored in `password` field; obfs defaults to "salamander"; peer parameter sets TLS server_name.
  - When adding new protocols, follow the `handle_*()` pattern to keep `execute()` clean.

- **Examples you can use in patches/tests:**
  - Shadowsocks: `sudo kiki set "ss://YWVzLTI1Ni1jZmI6cGFzc0BleGFtcGxlLmNvbTo4MDgw"`.
  - VMess: `sudo kiki set "vmess://ew0KICAidiI6ICIyIiwNCiAgImFkZCI6ICJoZWFydGJlYXQueXl5ZC5kZSIsDQogICJwb3J0IjogIjE0Njc4IiwNCiAgImlkIjogIjJkMzZlNDdhLTZjNjctNDUzNC1mYTNmLWIyYjQ2ZjJlMzNmMSINCn0="`.
  - Trojan: `sudo kiki set "trojan://password@example.com:443"`.
  - VLESS: `sudo kiki set "vless://uuid@example.com:443?security=tls&sni=example.com"`.
  - Hysteria2: `sudo kiki set "hysteria2://uuid@example.com:443?peer=example.com&obfs=salamander"` or `sudo kiki set "hy2://uuid@example.com:443"`.
  - Validation call (used by `check`): `sing-box check -c /etc/sing-box/config.json` — inspect stderr for validation details.
  - Logs: `sudo kiki logs` (show recent logs) and `sudo kiki logs -f` (follow new logs in real-time).

- **When making fixes / PRs:**
  - Test logic paths that decode both fully-encoded and partially-encoded `ss://` strings.
  - If adding logging, follow the existing pattern (simple println/eprintln, emoji UI) to keep CLI UX consistent.
  - Avoid hardcoding alternate config paths unless adding a CLI flag; existing code assumes `/etc/sing-box/config.json`.

If anything here is missing or unclear, tell me which area you want expanded (parsing edge-cases, service integration, or build/test commands) and I will update this file.
