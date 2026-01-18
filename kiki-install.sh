#!/bin/bash

# 定义颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}开始安装 KiKi 管理工具及 sing-box...${NC}"

# 1. 安装指定版本的 sing-box
echo -e "${GREEN}正在安装 sing-box v1.12.17...${NC}"
curl -fsSL https://sing-box.app/install.sh | sh -s -- --version 1.12.17

# 2. 准备目录结构
echo -e "${GREEN}正在准备配置目录...${NC}"
mkdir -p /etc/sing-box
mkdir -p /etc/kiki

# 3. 下载并替换 sing-box 配置文件
echo -e "${GREEN}正在从 GitHub 获取预设配置文件...${NC}"
# 注意：使用 raw 链接才能下载纯文本内容
CONFIG_URL="https://raw.githubusercontent.com/JamesShaw777/KiKi-A-SingBox-Management-Cli-/main/config.json"
curl -Lo /etc/sing-box/config.json $CONFIG_URL

if [ $? -eq 0 ]; then
    echo -e "${GREEN}=> 配置文件已就绪: /etc/sing-box/config.json${NC}"
else
    echo -e "${RED}=> 配置文件下载失败，请检查网络连接${NC}"
    exit 1
fi

# 4. 下载并安装 KiKi CLI
echo -e "${GREEN}正在从 Release 下载 KiKi 工具...${NC}"
KIKI_URL="https://github.com/JamesShaw777/KiKi-A-SingBox-Management-Cli-/releases/download/v0.1.0/tar.gz.tar.gz"
TEMP_DIR=$(mktemp -d)

curl -Lo $TEMP_DIR/kiki.tar.gz $KIKI_URL
tar -xzf $TEMP_DIR/kiki.tar.gz -C $TEMP_DIR

# 假设解压后二进制文件名叫 kiki，将其移动到目标目录
# 如果解压出的文件名不同，请相应修改
if [ -f "$TEMP_DIR/kiki" ]; then
    mv $TEMP_DIR/kiki /etc/kiki/kiki
    chmod +x /etc/kiki/kiki
    echo -e "${GREEN}=> KiKi 已安装至 /etc/kiki/kiki${NC}"
else
    # 兼容性处理：如果解压出来名字不对，尝试寻找可执行文件
    find $TEMP_DIR -type f -executable -exec mv {} /etc/kiki/kiki \;
    chmod +x /etc/kiki/kiki
fi

# 5. 添加到系统环境变量
echo -e "${GREEN}正在添加环境变量...${NC}"
if [[ ":$PATH:" != *":/etc/kiki:"* ]]; then
    # 针对当前会话添加
    export PATH=$PATH:/etc/kiki
    # 针对 Bash 永久添加
    echo 'export PATH=$PATH:/etc/kiki' >> ~/.bashrc
    # 针对 Zsh 永久添加 (如果存在)
    [ -f ~/.zshrc ] && echo 'export PATH=$PATH:/etc/kiki' >> ~/.zshrc
fi

# 6. 创建软连接到 /usr/local/bin (推荐做法，比改 PATH 更稳)
ln -sf /etc/kiki/kiki /usr/local/bin/kiki

echo -e "${BLUE}--- 安装完成 ---${NC}"
echo -e "你可以现在输入 ${GREEN}kiki check${NC} 来测试环境。"
echo -e "或者使用 ${GREEN}kiki set <URL>${NC} 来配置节点。"