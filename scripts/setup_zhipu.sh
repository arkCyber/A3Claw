#!/bin/bash

# 智谱AI配置脚本
# 用于快速配置智谱AI API到OpenClaw+

set -e

echo "🚀 配置智谱AI到OpenClaw+..."

# API密钥
ZHIPU_API_KEY="3a27cba615f24a979fef006b3cb2487f.mXxpSoA7Vd0NDyUk"

# 检查是否已设置环境变量
if [ -z "$ZHIPU_API_KEY" ]; then
    echo "⚠️  环境变量 ZHIPU_API_KEY 未设置"
    echo "📝 正在设置环境变量..."
    
    # 添加到 ~/.bashrc
    echo "export ZHIPU_API_KEY=\"3a27cba615f24a979fef006b3cb2487f.mXxpSoA7Vd0NDyUk\"" >> ~/.bashrc
    
    # 添加到 ~/.zshrc (如果存在)
    if [ -f ~/.zshrc ]; then
        echo "export ZHIPU_API_KEY=\"3a27cba615f24a979fef006b3cb2487f.mXxpSoA7Vd0NDyUk\"" >> ~/.zshrc
    fi
    
    # 设置当前会话的环境变量
    export ZHIPU_API_KEY="3a27cba615f24a979fef006b3cb2487f.mXxpSoA7Vd0NDyUk"
    
    echo "✅ 环境变量设置完成"
else
    echo "✅ 环境变量 ZHIPU_API_KEY 已设置"
fi

# 检查配置文件
CONFIG_FILE="config/servers.toml"
if [ ! -f "$CONFIG_FILE" ]; then
    echo "❌ 配置文件 $CONFIG_FILE 不存在"
    exit 1
fi

echo "📋 检查配置文件中的智谱AI配置..."

# 检查是否已存在智谱AI配置
if grep -q "zhipu-cloud" "$CONFIG_FILE"; then
    echo "✅ 智谱AI配置已存在"
    
    # 询问是否启用
    read -p "是否启用智谱AI服务？(y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        # 启用智谱AI
        sed -i.bak 's/enabled = false/enabled = true/' "$CONFIG_FILE"
        echo "✅ 智谱AI已启用"
    fi
else
    echo "⚠️  智谱AI配置不存在，请手动添加到 $CONFIG_FILE"
    echo "配置内容："
    cat << EOF

[[servers]]
id = "zhipu-cloud"
name = "智谱AI (云端)"
type = "zhipu"
endpoint = "https://open.bigmodel.cn/api/paas/v4"
port = 443
enabled = true
auto_start = false
model = "glm-4-flash"

EOF
fi

# 测试API连接
echo "🧪 测试智谱AI API连接..."
RESPONSE=$(curl -s -X POST "https://open.bigmodel.cn/api/paas/v4/chat/completions" \
  -H "Authorization: Bearer $ZHIPU_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-4-flash",
    "messages": [{"role": "user", "content": "Hello"}],
    "max_tokens": 10
  }' 2>/dev/null)

if echo "$RESPONSE" | grep -q '"choices"'; then
    echo "✅ 智谱AI API连接测试成功"
else
    echo "❌ 智谱AI API连接测试失败"
    echo "响应: $RESPONSE"
    exit 1
fi

# 编译项目
echo "🔨 编译OpenClaw+..."
if cargo build --release; then
    echo "✅ 编译成功"
else
    echo "❌ 编译失败"
    exit 1
fi

echo ""
echo "🎉 智谱AI配置完成！"
echo ""
echo "📋 下一步："
echo "1. 运行: ./target/release/openclaw-ui"
echo "2. 在AI设置中选择'智谱AI (云端)'"
echo "3. 选择模型: glm-4-flash 或 glm-4"
echo "4. 开始对话"
echo ""
echo "📚 更多信息请查看: docs/ZHIPU_AI_SETUP.md"
