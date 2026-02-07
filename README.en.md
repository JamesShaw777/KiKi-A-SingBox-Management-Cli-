# KiKi - A Sing-Box Management CLI

[简体中文](README.md) | English

KiKi is a lightweight Rust-based Linux CLI that simplifies daily `sing-box` management. It can parse node URLs, run environment diagnostics, and control the service.

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/badge/release-v0.1.0-green.svg)](https://github.com/JamesShaw777/KiKi-A-SingBox-Management-Cli-/releases)

---

## Features

- One-command setup: parse node URLs and update the `sing-box` config automatically.
- Environment checks: verify `sing-box` install, config path, and JSON validity.
- Service control: start/stop/restart via Systemd with simple commands.
- CN-optimized routing: built-in direct/proxy split rules for mainland China.

## Quick Install

Run the following to install `sing-box` (v1.12.17) and KiKi:

```bash
curl -fsSL https://raw.githubusercontent.com/JamesShaw777/KiKi-A-SingBox-Management-Cli-/main/kiki-install.sh | sudo bash
```

## Usage

After installation, you can use the `kiki` command anywhere.

### 1. Check environment

```bash
sudo kiki check
```

### 2. Set proxy node

KiKi supports multiple proxy protocols. Copy your node URL and run:

**Shadowsocks**

```bash
sudo kiki set "ss://YWVzLTI1Ni1jZmI6S1NYTmhuWnBqd0M2UGM2Q0E1NC4xNjkuMzUuMjI4OjMxNDQ0"
```

**VMess**

```bash
sudo kiki set "vmess://ew0KICAidiI6ICIyIiwNCiAgInBzIjogIk5MIiwNCiAgImFkZCI6ICJoZWFydGJlYXQueXl5ZC5kZSIsDQogICJwb3J0IjogIjE0Njc4IiwNCiAgImlkIjogIjJkMzZlNDdhLTZjNjctNDUzNC1mYTNmLWIyYjQ2ZjJlMzNmMSINCn0="
```

**Trojan**

```bash
sudo kiki set "trojan://password@example.com:443"
```

**VLESS**

```bash
sudo kiki set "vless://uuid@example.com:443?security=tls&sni=example.com"
```

**Hysteria2**

```bash
sudo kiki set "hysteria2://550e8400-e29b-41d4-a716-446655440000@example.com:443?peer=example.com&insecure=1&obfs=salamander"
```

Or use the `hy2://` prefix:

```bash
sudo kiki set "hy2://550e8400-e29b-41d4-a716-446655440000@example.com:443?peer=example.com"
```

**TUIC**

```bash
sudo kiki set "tuic://550e8400-e29b-41d4-a716-446655440000:password@example.com:443?sni=example.com&congestion_control=bbr&udp_relay_mode=native"
```

**AnyTLS**

```bash
sudo kiki set "anytls://password@example.com:443"
```

### 3. Manage service

```bash
sudo kiki start    # start sing-box
sudo kiki restart  # restart to apply config changes
sudo kiki stop     # stop sing-box
```

### 4. View logs

Show recent `sing-box` logs:

```bash
sudo kiki logs
```

Follow new logs in real time:

```bash
sudo kiki logs -f
```

Press `Ctrl+C` to exit.

## Project structure

```
.
├── src/
│   ├── main.rs          # CLI entry
│   └── commands/        # subcommands
│       ├── mod.rs
│       ├── set.rs       # URL parsing & JSON update
│       ├── check.rs     # environment check
│       └── service.rs   # Systemd control
├── config.json          # sing-box config template
└── kiki-install.sh      # install script
```

## Config notes

KiKi manages the config file at `/etc/sing-box/config.json`, which includes:

- DNS split: AliDNS for CN, Google DNS (DoT) for overseas.
- Routing rules: `geoip-cn` + `geosite-cn` for direct CN traffic.

## Contributing

Issues and pull requests are welcome.
