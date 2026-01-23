#!/bin/bash

# 定义颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

VERSION="1.12.17"

echo -e "${BLUE}开始安装 KiKi 管理工具及 sing-box v${VERSION}...${NC}"

# --- 1. 系统环境自动识别 ---
echo -e "${GREEN}正在检测系统环境...${NC}"

# 检测架构
ARCH=$(uname -m)
case ${ARCH} in
    x86_64)  SB_ARCH="amd64" ;;
    aarch64) SB_ARCH="arm64" ;;
    armv7l)  SB_ARCH="armv7" ;;
    *) echo -e "${RED}不支持的架构: ${ARCH}${NC}"; exit 1 ;;
esac

# 检测包管理器
if command -v dpkg >/dev/null 2>&1; then
    PKG_TYPE="deb"
    PKG_MANAGER="dpkg -i"
    # 注意：sing-box 的 deb 文件名格式通常是 sing-box_1.12.17_linux_amd64.deb
    FILENAME="sing-box_${VERSION}_linux_${SB_ARCH}.deb"
elif command -v rpm >/dev/null 2>&1; then
    PKG_TYPE="rpm"
    PKG_MANAGER="rpm -Uvh"
    # 注意：sing-box 的 rpm 文件名格式通常是 sing-box-1.12.17-linux-amd64.rpm
    FILENAME="sing-box-${VERSION}-linux-${SB_ARCH}.rpm"
else
    echo -e "${RED}无法识别的系统类型（非 Debian/RPM 系）${NC}"
    exit 1
fi

# --- 2. 下载并安装 sing-box ---
URL="https://cdn.gh-proxy.org/https://github.com/SagerNet/sing-box/releases/download/v${VERSION}/${FILENAME}"
echo -e "${GREEN}正在从 GitHub 下载: ${FILENAME}${NC}"

TEMP_DIR=$(mktemp -d)
curl -Lo "${TEMP_DIR}/${FILENAME}" "${URL}"

if [ $? -ne 0 ]; then
    echo -e "${RED}下载 sing-box 失败，请检查网络或版本号是否存在。${NC}"
    exit 1
fi

echo -e "${GREEN}正在执行安装...${NC}"
sudo ${PKG_MANAGER} "${TEMP_DIR}/${FILENAME}"

# --- 3. 准备目录结构 ---
echo -e "${GREEN}正在准备配置目录...${NC}"
sudo mkdir -p /etc/sing-box
sudo mkdir -p /etc/kiki

# --- 4. 下载配置文件 ---
CONFIG_URL="https://cdn.gh-proxy.org/https://raw.githubusercontent.com/JamesShaw777/KiKi-A-SingBox-Management-Cli-/main/config.json"
sudo curl -Lo /etc/sing-box/config.json "${CONFIG_URL}"

# --- 5. 下载并安装 KiKi CLI ---
echo -e "${GREEN}正在下载 KiKi 工具...${NC}"
KIKI_URL="https://cdn.gh-proxy.org/https://github.com/JamesShaw777/KiKi-A-SingBox-Management-Cli-/releases/download/v0.1.0/tar.gz.tar.gz"

curl -Lo "${TEMP_DIR}/kiki.tar.gz" "${KIKI_URL}"
tar -xzf "${TEMP_DIR}/kiki.tar.gz" -C "${TEMP_DIR}"

# 移动二进制文件（带兼容性逻辑）
KIKI_BIN=$(find "${TEMP_DIR}" -type f -name "kiki" | head -n 1)
if [ -z "$KIKI_BIN" ]; then
    # 如果找不到叫 kiki 的，找第一个可执行文件
    KIKI_BIN=$(find "${TEMP_DIR}" -type f -executable | head -n 1)
fi

if [ -n "$KIKI_BIN" ]; then
    sudo mv "$KIKI_BIN" /etc/kiki/kiki
    sudo chmod +x /etc/kiki/kiki
    sudo ln -sf /etc/kiki/kiki /usr/local/bin/kiki
    echo -e "${GREEN}=> KiKi 已安装至 /usr/local/bin/kiki${NC}"
else
    echo -e "${RED}未在压缩包内找到可执行文件${NC}"
    exit 1
fi

# 清理临时文件
rm -rf "${TEMP_DIR}"

echo -e "${BLUE}--- 安装完成 ---${NC}"
echo -e "你可以现在输入 ${GREEN}kiki check${NC} 来测试环境。"