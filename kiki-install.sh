#!/bin/bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

REPO="JamesShaw777/KiKi-A-SingBox-Management-Cli-"
SINGBOX_VERSION="${SINGBOX_VERSION:-1.12.17}"
GITHUB_PROXY="${GITHUB_PROXY:-https://cdn.gh-proxy.org/}"
KIKI_TAG="${KIKI_TAG:-}"
KIKI_TARGET="${KIKI_TARGET:-}"
KIKI_LIBC="${KIKI_LIBC:-}"

log_info() {
    echo -e "${GREEN}$*${NC}"
}

log_error() {
    echo -e "${RED}$*${NC}" >&2
}

run_root() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    elif command -v sudo >/dev/null 2>&1; then
        sudo "$@"
    else
        log_error "当前用户不是 root，且系统中没有 sudo。"
        exit 1
    fi
}

proxy_url() {
    printf '%s%s' "${GITHUB_PROXY}" "$1"
}

download_file() {
    local url="$1"
    local destination="$2"
    curl -fL --retry 3 --retry-delay 1 -o "${destination}" "${url}"
}

resolve_latest_tag() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' \
        | head -n 1
}

detect_libc() {
    if [ -n "${KIKI_LIBC}" ]; then
        case "${KIKI_LIBC}" in
            gnu|musl)
                printf '%s\n' "${KIKI_LIBC}"
                return
                ;;
            *)
                log_error "不支持的 KIKI_LIBC: ${KIKI_LIBC}，可选值为 gnu 或 musl。"
                exit 1
                ;;
        esac
    fi

    if [ -f /etc/alpine-release ]; then
        printf 'musl\n'
        return
    fi

    if command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | head -n 1 | grep -qi musl; then
        printf 'musl\n'
        return
    fi

    printf 'gnu\n'
}

log_info "开始安装 KiKi 管理工具及 sing-box v${SINGBOX_VERSION}..."
log_info "正在检测系统环境..."

ARCH="$(uname -m)"
LIBC="$(detect_libc)"

case "${ARCH}" in
    x86_64|amd64)
        SB_ARCH="amd64"
        KIKI_ARCH="x86_64"
        ;;
    aarch64|arm64)
        SB_ARCH="arm64"
        KIKI_ARCH="aarch64"
        ;;
    armv7l|armv7|armv7hl)
        SB_ARCH="armv7"
        KIKI_ARCH="armv7"
        ;;
    *)
        log_error "不支持的架构: ${ARCH}"
        exit 1
        ;;
esac

case "${LIBC}" in
    gnu)
        case "${KIKI_ARCH}" in
            x86_64) DEFAULT_KIKI_TARGET="x86_64-unknown-linux-gnu" ;;
            aarch64) DEFAULT_KIKI_TARGET="aarch64-unknown-linux-gnu" ;;
            armv7) DEFAULT_KIKI_TARGET="armv7-unknown-linux-gnueabihf" ;;
        esac
        ;;
    musl)
        case "${KIKI_ARCH}" in
            x86_64) DEFAULT_KIKI_TARGET="x86_64-unknown-linux-musl" ;;
            aarch64) DEFAULT_KIKI_TARGET="aarch64-unknown-linux-musl" ;;
            armv7) DEFAULT_KIKI_TARGET="armv7-unknown-linux-musleabihf" ;;
        esac
        ;;
    *)
        log_error "不支持的 libc 类型: ${LIBC}"
        exit 1
        ;;
esac

KIKI_TARGET="${KIKI_TARGET:-${DEFAULT_KIKI_TARGET}}"

if [ -z "${KIKI_TAG}" ]; then
    KIKI_TAG="$(resolve_latest_tag || true)"
fi

if [ -z "${KIKI_TAG}" ]; then
    log_error "无法自动获取最新 KiKi 版本号，请手动设置 KIKI_TAG，例如 KIKI_TAG=v0.2.0。"
    exit 1
fi

log_info "检测到架构: ${ARCH}"
log_info "检测到 libc: ${LIBC}"
log_info "将安装 KiKi 版本: ${KIKI_TAG}"
log_info "将下载 KiKi 目标: ${KIKI_TARGET}"

if command -v dpkg >/dev/null 2>&1; then
    FILENAME="sing-box_${SINGBOX_VERSION}_linux_${SB_ARCH}.deb"
    PKG_INSTALL_CMD=(dpkg -i)
elif command -v rpm >/dev/null 2>&1; then
    FILENAME="sing-box-${SINGBOX_VERSION}-linux-${SB_ARCH}.rpm"
    PKG_INSTALL_CMD=(rpm -Uvh)
else
    log_error "无法识别的系统类型（非 Debian/RPM 系）"
    exit 1
fi

TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TEMP_DIR}"' EXIT

SINGBOX_URL="$(proxy_url "https://github.com/SagerNet/sing-box/releases/download/v${SINGBOX_VERSION}/${FILENAME}")"
CONFIG_URL="$(proxy_url "https://raw.githubusercontent.com/${REPO}/${KIKI_TAG}/config.json")"
KIKI_ARCHIVE="kiki-${KIKI_TAG}-${KIKI_TARGET}.tar.gz"
KIKI_URL="$(proxy_url "https://github.com/${REPO}/releases/download/${KIKI_TAG}/${KIKI_ARCHIVE}")"

log_info "正在从 GitHub 下载 sing-box: ${FILENAME}"
download_file "${SINGBOX_URL}" "${TEMP_DIR}/${FILENAME}"

log_info "正在安装 sing-box..."
run_root "${PKG_INSTALL_CMD[@]}" "${TEMP_DIR}/${FILENAME}"

log_info "正在准备配置目录..."
run_root mkdir -p /etc/sing-box /etc/kiki

log_info "正在下载配置文件..."
run_root curl -fL --retry 3 --retry-delay 1 -o /etc/sing-box/config.json "${CONFIG_URL}"

log_info "正在下载 KiKi 工具..."
download_file "${KIKI_URL}" "${TEMP_DIR}/${KIKI_ARCHIVE}"

mkdir -p "${TEMP_DIR}/kiki-extract"
tar -xzf "${TEMP_DIR}/${KIKI_ARCHIVE}" -C "${TEMP_DIR}/kiki-extract"

KIKI_BIN="$(find "${TEMP_DIR}/kiki-extract" -type f -name "kiki" | head -n 1)"
if [ -z "${KIKI_BIN}" ]; then
    KIKI_BIN="$(find "${TEMP_DIR}/kiki-extract" -type f -perm -111 | head -n 1)"
fi

if [ -z "${KIKI_BIN}" ]; then
    log_error "未在压缩包内找到可执行文件。"
    exit 1
fi

run_root install -m 0755 "${KIKI_BIN}" /etc/kiki/kiki
run_root ln -sf /etc/kiki/kiki /usr/local/bin/kiki

echo -e "${BLUE}--- 安装完成 ---${NC}"
echo -e "你可以现在输入 ${GREEN}kiki check${NC} 来测试环境。"
