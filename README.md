# KiKi - A Sing-Box Management CLI

🚀 **KiKi** 是一个用 Rust 编写的轻量级 Linux 命令行工具，旨在简化 `sing-box` 的日常管理。它提供了快速解析节点、环境诊断以及服务控制功能。

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/badge/release-v0.1.0-green.svg)](https://github.com/JamesShaw777/KiKi-A-SingBox-Management-Cli-/releases)

---

## ✨ 功能特性

- **一键设点**：支持解析 `ss://` 链接并自动更新 `sing-box` 配置文件。
- **环境诊断**：自动检查 `sing-box` 是否安装、配置路径是否正确以及 JSON 语法校验。
- **服务管理**：集成 Systemd，通过简单命令启动、停止或重启服务。
- **中国境内优化**：默认配套针对国内直连、国外代理分流优化的配置方案。

## 🛠️ 快速安装

在终端执行以下命令，即可完成 `sing-box` (v1.12.17) 与 **KiKi** 的自动化安装：

```bash
curl -fsSL https://raw.githubusercontent.com/JamesShaw777/KiKi-A-SingBox-Management-Cli-/main/kiki-install.sh | sudo bash
```

## 📖 使用指南

安装完成后，你可以在任何地方直接使用 `kiki` 命令。

### 1. 检查运行环境

在启动前，建议运行诊断确认一切就绪：

```bash
sudo kiki check
```

### 2. 设置代理节点

KiKi 支持多种代理协议。复制你的订阅链接并运行：

**Shadowsocks 节点：**

```bash
sudo kiki set "ss://YWVzLTI1Ni1jZmI6S1NYTmhuWnBqd0M2UGM2Q0E1NC4xNjkuMzUuMjI4OjMxNDQ0"
```

**VMess 节点：**

```bash
sudo kiki set "vmess://ew0KICAidiI6ICIyIiwNCiAgInBzIjogIk5MIiwNCiAgImFkZCI6ICJoZWFydGJlYXQueXl5ZC5kZSIsDQogICJwb3J0IjogIjE0Njc4IiwNCiAgImlkIjogIjJkMzZlNDdhLTZjNjctNDUzNC1mYTNmLWIyYjQ2ZjJlMzNmMSINCn0="
```

**Trojan 节点：**

```bash
sudo kiki set "trojan://password@example.com:443"
```

**VLESS 节点：**

```bash
sudo kiki set "vless://uuid@example.com:443?security=tls&sni=example.com"
```

**Hysteria2 节点：**

```bash
sudo kiki set "hysteria2://550e8400-e29b-41d4-a716-446655440000@example.com:443?peer=example.com&insecure=1&obfs=salamander"
```

或使用 `hy2://` 前缀：

```bash
sudo kiki set "hy2://550e8400-e29b-41d4-a716-446655440000@example.com:443?peer=example.com"
```

**AnyTLS 节点：**

```bash
sudo kiki set "anytls://password@example.com:443"
```

### 3. 管理服务状态

```bash
sudo kiki start    # 启动 sing-box
sudo kiki restart  # 重启以应用配置更改
sudo kiki stop     # 停止服务
```

### 4. 查看日志

查看最近的 sing-box 日志：

```bash
sudo kiki logs
```

实时跟踪新日志（类似 `tail -f`）：

```bash
sudo kiki logs -f
```

按 `Ctrl+C` 退出实时跟踪模式。

## 📂 项目结构

```
.
├── src/
│   ├── main.rs          # CLI 参数入口
│   └── commands/        # 子命令逻辑实现
│       ├── mod.rs
│       ├── set.rs       # 节点解析与 JSON 修改
│       ├── check.rs     # 系统环境诊断
│       └── service.rs   # Systemd 服务控制
├── config.json          # 预设的 sing-box 配置文件模板
└── kiki-install.sh      # 自动化安装脚本
```

## ⚙️ 配置文件说明

KiKi 默认管理的配置文件位于：`/etc/sing-box/config.json`。 其内置了：

- **DNS 分流**：国内使用阿里云 DNS，国外使用 Google DNS (DoT)。
- **路由规则**：自动识别 `geoip-cn` 和 `geosite-cn` 实现国内流量直连。

## 🤝 贡献与反馈

如果你在使用过程中遇到问题，欢迎提交 Issues 或 Pull Request。
