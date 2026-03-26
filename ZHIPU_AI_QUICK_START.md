# 🚀 智谱AI快速开始指南

## 一键配置

```bash
# 进入OpenClaw+目录
cd /Users/arkSong/workspace/OpenClaw+

# 运行配置脚本
./scripts/setup_zhipu.sh
```

## 手动配置

### 1. 设置环境变量

```bash
export ZHIPU_API_KEY="3a27cba615f24a979fef006b3cb2487f.mXxpSoA7Vd0NDyUk"
```

### 2. 启用智谱AI服务

编辑 `config/servers.toml`，将 `enabled = false` 改为 `enabled = true`：

```toml
[[servers]]
id = "zhipu-cloud"
name = "智谱AI (云端)"
type = "zhipu"
endpoint = "https://open.bigmodel.cn/api/paas/v4"
port = 443
enabled = true  # 改为 true
auto_start = false
model = "glm-4-flash"
```

### 3. 编译运行

```bash
cargo build --release
./target/release/openclaw-ui
```

## 测试API

```bash
# 测试连接
./scripts/test_zhipu.sh
```

## 使用方法

### 在UI中
1. 打开OpenClaw+界面
2. 进入"AI设置"
3. 选择"智谱AI (云端)"
4. 选择模型：
   - `glm-4-flash` - 快速响应
   - `glm-4` - 高性能
5. 开始对话

### 支持的功能
- ✅ 文本生成
- ✅ 工具调用
- ✅ ReAct循环
- ✅ 多轮对话
- ✅ 错误处理

## 故障转移

智谱AI已配置在故障转移链中，当本地服务不可用时自动切换：

1. Ollama (本地)
2. llama.cpp (本地)
3. **智谱AI (云端)** ←
4. DeepSeek (云端)
5. OpenAI (云端)

## 模型对比

| 模型 | 速度 | 质量 | 成本 | 适用场景 |
|------|------|------|------|----------|
| glm-4-flash | ⚡ 快 | 🟢 好 | 💰 低 | 实时对话 |
| glm-4 | 🐌 慢 | 🔴 优 | 💸 高 | 复杂任务 |

## 注意事项

- 需要网络连接到 `open.bigmodel.cn`
- API有调用配额限制
- 按使用量计费
- 请妥善保管API密钥

---

**配置完成！享受智谱AI的强大能力！** 🎉
