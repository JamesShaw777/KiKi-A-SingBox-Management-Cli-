# KiKi

[简体中文](README.md) | English

KiKi is a Linux CLI for `sing-box`, focused on node import, configuration validation, and service control.

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/JamesShaw777/kiki)](https://github.com/JamesShaw777/kiki/releases)
[![GitHub Stars](https://img.shields.io/github/stars/JamesShaw777/kiki?style=social)](https://github.com/JamesShaw777/kiki/stargazers)

---

## Features

- Multi-protocol node import: supports `ss://`, `vmess://`, `trojan://`, `vless://`, `hysteria2://`, `hy2://`, `tuic://`, and `anytls://`.
- Environment checks: verify `sing-box` install, config path, and JSON validity.
- Service control: start/stop/restart via Systemd with simple commands.
- CN-optimized routing: built-in direct/proxy split rules for mainland China.

## Quick Install

Run the following to install `sing-box` (v1.12.17) and KiKi:

```bash
curl -fsSL https://cdn.gh-proxy.org/https://raw.githubusercontent.com/JamesShaw777/kiki/main/kiki-install.sh | sudo bash
```

If you are already running as `root`, replace `sudo bash` with `bash`.

The installer auto-detects `x86_64`, `aarch64`, `armv7` and `gnu`/`musl`, then downloads the matching release asset.

If you want to pin a release or force a target explicitly:

```bash
curl -fsSL https://cdn.gh-proxy.org/https://raw.githubusercontent.com/JamesShaw777/kiki/main/kiki-install.sh | sudo env KIKI_TAG=v0.2.0 KIKI_TARGET=x86_64-unknown-linux-musl bash
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

## Acknowledgements

KiKi is built around the configuration model and protocol capabilities provided by [`sing-box`](https://github.com/SagerNet/sing-box). Field mapping, node parsing behavior, and config validation in this project are aligned with the official `sing-box` documentation and implementation where applicable.

Thanks to the `sing-box` project and its maintainers for providing a solid proxy core and high-quality documentation.

This repository is an independent community tool and is not an official `sing-box` project.

## Support

If KiKi is useful for your workflow, consider giving the repository a `Star` on GitHub. It helps more users discover the project and supports ongoing maintenance.

## Contributing

Issues and pull requests are welcome.
