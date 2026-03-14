# 树莓派部署指南

## 硬件要求

### 推荐配置

- **树莓派 4/5 (8GB RAM)** - 可运行本地推理
- **树莓派 4 (4GB RAM)** - 可运行小型模型或使用云服务
- **存储**: 至少 32GB microSD 卡（推荐 64GB+ SSD）
- **操作系统**: Raspberry Pi OS 64-bit (Debian Bookworm)

### 性能对比

| 配置 | 内存 | 本地推理能力 | 推荐模型 |
|------|------|------------|---------|
| 树莓派 5 8GB | 8GB | ✅ 优秀 | Qwen2.5:3B, Phi-3.5:mini, Gemma2:2B |
| 树莓派 4 8GB | 8GB | ✅ 良好 | Qwen2.5:3B, Phi-3.5:mini, Gemma2:2B |
| 树莓派 4 4GB | 4GB | ⚠️ 有限 | TinyLlama:1.1B, Phi-2:2.7B (量化) |
| 树莓派 4 2GB | 2GB | ❌ 不推荐 | 仅云服务 |

## 部署方案

### 方案 A: 本地推理 (8GB 树莓派推荐)

**优点**:
- 完全离线工作
- 无 API 费用
- 数据隐私保护
- 低延迟响应

**适合的模型**:

1. **Qwen2.5:3B** (推荐)
   - 模型大小: ~2GB
   - 内存占用: ~3-4GB
   - 性能: 优秀的中英文能力
   - 速度: ~5-10 tokens/s

2. **Phi-3.5:mini** (3.8B)
   - 模型大小: ~2.3GB
   - 内存占用: ~3.5-4.5GB
   - 性能: 优秀的推理能力
   - 速度: ~4-8 tokens/s

3. **Gemma2:2B**
   - 模型大小: ~1.6GB
   - 内存占用: ~2.5-3GB
   - 性能: 快速响应
   - 速度: ~8-15 tokens/s

### 方案 B: 云服务 (所有树莓派)

**优点**:
- 支持大型模型 (GPT-4, Claude, etc.)
- 更快的响应速度
- 更低的硬件要求

**支持的云服务**:
- OpenAI API (GPT-4, GPT-3.5)
- Anthropic Claude
- Google Gemini
- 阿里云通义千问
- 智谱 AI (GLM-4)

### 方案 C: 混合模式 (推荐)

- **本地**: 快速任务、隐私数据处理
- **云端**: 复杂任务、需要大模型的场景

## 安装步骤

### 1. 准备树莓派环境

```bash
# 更新系统
sudo apt update && sudo apt upgrade -y

# 安装必要的依赖
sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    cmake \
    git \
    curl

# 安装 Rust (ARM64)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. 安装 Ollama (本地推理)

```bash
# 下载 Ollama for ARM64
curl -fsSL https://ollama.com/install.sh | sh

# 启动 Ollama 服务
sudo systemctl start ollama
sudo systemctl enable ollama

# 下载推荐模型 (选择一个)
ollama pull qwen2.5:3b      # 推荐: 3GB, 中英文优秀
ollama pull phi3.5:mini     # 备选: 2.3GB, 推理能力强
ollama pull gemma2:2b       # 轻量: 1.6GB, 速度快

# 测试模型
ollama run qwen2.5:3b "你好，请介绍一下自己"
```

### 3. 编译 OpenClaw+ for ARM64

#### 选项 A: 在树莓派上直接编译

```bash
# 克隆代码
git clone https://github.com/yourusername/OpenClaw+.git
cd OpenClaw+

# 编译 (需要 30-60 分钟)
cargo build --release -p openclaw-ui

# 二进制文件位置
ls -lh target/release/openclaw-plus
```

#### 选项 B: 交叉编译 (在 macOS/Linux 上)

```bash
# 安装 ARM64 交叉编译工具链
rustup target add aarch64-unknown-linux-gnu

# 安装交叉编译器 (macOS)
brew install aarch64-unknown-linux-gnu

# 或者 (Linux)
sudo apt install gcc-aarch64-linux-gnu

# 配置 Cargo
cat >> ~/.cargo/config.toml << 'EOF'
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
EOF

# 编译
cargo build --release --target aarch64-unknown-linux-gnu -p openclaw-ui

# 传输到树莓派
scp target/aarch64-unknown-linux-gnu/release/openclaw-plus pi@raspberrypi.local:~/
```

### 4. 配置 OpenClaw+

```bash
# 创建配置目录
mkdir -p ~/.config/openclaw-plus

# 创建配置文件
cat > ~/.config/openclaw-plus/config.toml << 'EOF'
[openclaw_ai]
# 本地推理配置
provider = "Ollama"
endpoint = "http://localhost:11434"
model = "qwen2.5:3b"

# 或者使用云服务
# provider = "OpenAI"
# endpoint = "https://api.openai.com/v1"
# model = "gpt-3.5-turbo"
# api_key = "your-api-key-here"

[security]
workspace = "/home/pi/workspace"
allowed_hosts = ["localhost", "api.openai.com"]
max_memory_mb = 6144  # 为系统保留 2GB
EOF
```

### 5. 启动 OpenClaw+

```bash
# 直接运行 (无 GUI，仅终端)
./openclaw-plus

# 或者使用 systemd 服务
sudo tee /etc/systemd/system/openclaw.service << 'EOF'
[Unit]
Description=OpenClaw+ AI Assistant
After=network.target ollama.service

[Service]
Type=simple
User=pi
WorkingDirectory=/home/pi
ExecStart=/home/pi/openclaw-plus
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl start openclaw
sudo systemctl enable openclaw
```

## 性能优化

### 1. 内存优化

```bash
# 增加 swap (如果使用 4GB 树莓派)
sudo dphys-swapfile swapoff
sudo sed -i 's/CONF_SWAPSIZE=100/CONF_SWAPSIZE=4096/' /etc/dphys-swapfile
sudo dphys-swapfile setup
sudo dphys-swapfile swapon

# 限制 Ollama 内存使用
export OLLAMA_MAX_LOADED_MODELS=1
export OLLAMA_NUM_PARALLEL=1
```

### 2. CPU 优化

```bash
# 超频 (树莓派 4，需要良好散热)
sudo tee -a /boot/config.txt << 'EOF'
over_voltage=6
arm_freq=2000
gpu_freq=750
EOF

# 重启生效
sudo reboot
```

### 3. 存储优化

```bash
# 使用 SSD 而不是 microSD
# 1. 通过 USB 3.0 连接 SSD
# 2. 使用 Raspberry Pi Imager 将系统安装到 SSD
# 3. 从 SSD 启动 (树莓派 4/5 支持)
```

## 推理性能测试

### 测试脚本

```bash
#!/bin/bash
# test_inference.sh

echo "Testing Ollama inference performance..."

# 测试 1: 简单问答
echo -e "\n=== Test 1: Simple Q&A ==="
time curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:3b",
  "prompt": "What is the capital of France?",
  "stream": false
}' | jq -r '.response'

# 测试 2: 中文问答
echo -e "\n=== Test 2: Chinese Q&A ==="
time curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:3b",
  "prompt": "请用一句话介绍人工智能",
  "stream": false
}' | jq -r '.response'

# 测试 3: 代码生成
echo -e "\n=== Test 3: Code Generation ==="
time curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:3b",
  "prompt": "Write a Python function to calculate fibonacci numbers",
  "stream": false
}' | jq -r '.response'
```

### 预期性能 (树莓派 4/5 8GB)

| 模型 | 首次响应 | 生成速度 | 内存占用 |
|------|---------|---------|---------|
| Qwen2.5:3B | ~1-2s | 5-10 tokens/s | 3-4GB |
| Phi-3.5:mini | ~1-2s | 4-8 tokens/s | 3.5-4.5GB |
| Gemma2:2B | ~0.5-1s | 8-15 tokens/s | 2.5-3GB |

## 远程访问

### 通过 SSH 隧道

```bash
# 在本地机器上
ssh -L 8080:localhost:8080 pi@raspberrypi.local

# 然后访问 http://localhost:8080
```

### 通过 Tailscale (推荐)

```bash
# 在树莓派上安装 Tailscale
curl -fsSL https://tailscale.com/install.sh | sh
sudo tailscale up

# 获取 Tailscale IP
tailscale ip -4

# 从任何设备访问
# http://100.x.x.x:8080
```

## 故障排查

### 问题 1: 内存不足

**症状**: Ollama 崩溃或响应缓慢

**解决方案**:
```bash
# 使用更小的模型
ollama pull gemma2:2b

# 或者使用量化版本
ollama pull qwen2.5:3b-q4_0  # 4-bit 量化，更小内存
```

### 问题 2: 编译失败

**症状**: `cargo build` 出错

**解决方案**:
```bash
# 增加 swap
sudo dphys-swapfile swapoff
sudo sed -i 's/CONF_SWAPSIZE=.*/CONF_SWAPSIZE=8192/' /etc/dphys-swapfile
sudo dphys-swapfile setup
sudo dphys-swapfile swapon

# 使用单线程编译
cargo build --release -p openclaw-ui -j 1
```

### 问题 3: 推理速度慢

**症状**: 生成速度 < 3 tokens/s

**解决方案**:
```bash
# 1. 检查 CPU 频率
vcgencmd measure_clock arm

# 2. 检查温度 (应该 < 80°C)
vcgencmd measure_temp

# 3. 改善散热
# - 安装散热片
# - 使用风扇
# - 使用官方主动散热壳

# 4. 使用更小的模型
ollama pull gemma2:2b
```

## 生产部署建议

### 1. 使用 Docker (可选)

```bash
# 安装 Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker pi

# 运行 Ollama
docker run -d \
  --name ollama \
  --restart always \
  -v ollama:/root/.ollama \
  -p 11434:11434 \
  ollama/ollama:latest

# 拉取模型
docker exec ollama ollama pull qwen2.5:3b
```

### 2. 监控和日志

```bash
# 查看系统资源
htop

# 查看 Ollama 日志
sudo journalctl -u ollama -f

# 查看 OpenClaw+ 日志
sudo journalctl -u openclaw -f
```

### 3. 自动更新

```bash
# 创建更新脚本
cat > ~/update_openclaw.sh << 'EOF'
#!/bin/bash
cd ~/OpenClaw+
git pull
cargo build --release -p openclaw-ui
sudo systemctl restart openclaw
EOF

chmod +x ~/update_openclaw.sh

# 添加到 crontab (每周日凌晨 3 点更新)
(crontab -l 2>/dev/null; echo "0 3 * * 0 ~/update_openclaw.sh") | crontab -
```

## 总结

**8GB 树莓派完全可以运行 OpenClaw+ 本地推理！**

推荐配置:
- **硬件**: 树莓派 4/5 8GB + SSD
- **模型**: Qwen2.5:3B (中英文) 或 Gemma2:2B (速度优先)
- **内存**: 为模型预留 4GB，系统使用 4GB
- **存储**: 至少 32GB (推荐 64GB+ SSD)

预期性能:
- 首次响应: 1-2 秒
- 生成速度: 5-10 tokens/秒
- 完全离线工作
- 低功耗 (~15W)

这使得树莓派成为一个优秀的边缘 AI 设备！
