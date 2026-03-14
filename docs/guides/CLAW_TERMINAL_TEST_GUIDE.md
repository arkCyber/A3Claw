# Claw Terminal 自动测试指南

## 测试方法

UI 已经内置了完整的自动测试功能，包含你要求的所有测试用例。

### 方法 1: 使用内置自动测试（推荐）

1. **启动 OpenClaw+ UI**
   ```bash
   ./scripts/run.sh
   ```

2. **进入 Claw Terminal 页面**
   - 点击左侧导航栏的 "Claw Terminal"

3. **启动自动测试**
   - 点击页面右上角的 "🧪 Auto Test" 按钮
   - 系统会自动执行 10 个测试用例

### 内置测试用例清单

自动测试会依次执行以下 10 个测试用例：

| # | 测试名称 | 测试内容 | 对应你的要求 |
|---|---------|---------|-------------|
| 1 | 基本对话 | "你好！请用一句话介绍你自己" | ✅ "你好" |
| 2 | 天气查询 | "用 weather.get 查询北京今天的天气" | - |
| 3 | 网页搜索 | "用 search.web 搜索: OpenClaw AI agent" | ✅ "搜索 OpenClaw 相关信息" |
| 4 | 文件系统 | "列出 /tmp 目录内容，然后在 /tmp 创建 openclaw_test.txt 写入 'test ok'" | ✅ "列出当前目录" |
| 5 | Python执行 | "用 python.run 执行: print(sum(range(1,101)))" | - |
| 6 | 网页抓取 | "用 web.fetch 抓取 https://httpbin.org/json 并显示结果" | - |
| 7 | 知识库 | "用 knowledge.search 搜索: AI agent architecture" | - |
| 8 | 画布管理 | "用 canvas.create 创建画布并添加节点" | - |
| 9 | 安全状态 | "显示当前沙箱安全状态和权限配置" | - |
| 10 | 技能列表 | "列出你拥有的所有 skills 并分类显示" | - |

### 测试执行流程

1. **自动测试启动**
   ```
   🧪 Auto-Test Suite starting (10 core tests — waits for each to finish before proceeding)...
   ```

2. **逐个执行测试**
   ```
   🧪 [1/10] 基本对话
   → 发送: "你好！请用一句话介绍你自己"
   → 等待 AI 响应
   → 记录结果（成功/失败、响应时间）
   
   🧪 [2/10] 天气查询
   → 发送: "用 weather.get 查询北京今天的天气"
   → 等待 AI 响应
   → 记录结果
   
   ... (依次执行所有 10 个测试)
   ```

3. **测试完成**
   ```
   ✅ Auto-Test complete! 8/10 passed, avg 2.3s/test
   ```

### 预期结果

#### ✅ 成功的测试
- **基本对话**: AI 应该用一句话介绍自己
- **网页搜索**: 使用 search.web skill 搜索 OpenClaw 相关信息
- **文件系统**: 列出目录并创建测试文件
- **技能列表**: 显示所有可用的 skills

#### ⚠️ 可能失败的测试
- **天气查询**: 需要 weather API 配置
- **知识库**: 需要 RAG 数据库初始化
- **画布管理**: 需要 canvas 功能启用

### 测试结果查看

测试完成后，UI 会显示：
- ✅ 通过的测试数量
- ❌ 失败的测试数量
- ⏱️ 平均响应时间
- 📊 每个测试的详细结果

---

## 方法 2: 手动测试

如果你想手动测试，可以直接在 Claw Terminal 输入框中输入：

### 测试 1: 基本对话
```
你好
```
**预期**: AI 应该友好地回复并介绍自己

### 测试 2: 列出目录
```
列出当前目录
```
**预期**: AI 应该使用 file.list 或 shell 命令列出目录内容

### 测试 3: 搜索信息
```
搜索 OpenClaw 相关信息
```
**预期**: AI 应该使用 search.web skill 搜索并返回结果

---

## 验证配置

### 1. 检查模型配置
```bash
cat ~/Library/Application\ Support/openclaw-plus/config.toml | grep -A 5 openclaw_ai
```

**应该显示**:
```toml
[openclaw_ai]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "qwen2.5:7b"  # ✅ 必须是 qwen2.5:7b
```

### 2. 验证 Ollama 运行
```bash
curl http://localhost:11434/api/tags
```

**应该返回**:
```json
{
  "models": [
    {
      "name": "qwen2.5:7b",
      ...
    }
  ]
}
```

### 3. 检查 Plugin Gateway
```bash
curl http://localhost:7878/health
```

**应该返回**:
```json
{
  "status": "ok"
}
```

---

## 故障排查

### 问题 1: 测试无响应
**原因**: 模型配置错误或 Ollama 未运行
**解决**:
```bash
# 检查 Ollama
ollama list

# 重启 Ollama
ollama serve

# 验证模型
ollama run qwen2.5:7b "你好"
```

### 问题 2: 工具调用失败
**原因**: Plugin Gateway 未启动
**解决**:
```bash
# 检查 Gateway
ps aux | grep plugin-gateway

# 重启 UI（会自动启动 Gateway）
pkill -f OpenClawPlus
./scripts/run.sh
```

### 问题 3: 测试卡住
**原因**: 等待超时或推理循环未完成
**解决**:
- 点击 "⏹ Stop Test" 停止测试
- 检查 UI 日志（stderr 输出）
- 验证模型是否支持工具调用

---

## 测试报告示例

```
🧪 Auto-Test Suite Results
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Total Tests:     10
✅ Passed:       8
❌ Failed:       2
⏱️ Avg Time:     2.3s

Detailed Results:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[1/10] ✅ 基本对话                    1.2s
       Response: "你好！我是 OpenClaw，一个超级 AI 智能体..."

[2/10] ❌ 天气查询                    timeout
       Error: Weather API not configured

[3/10] ✅ 网页搜索                    3.5s
       Response: "找到 15 条关于 OpenClaw AI agent 的结果..."

[4/10] ✅ 文件系统                    2.1s
       Response: "已列出 /tmp 目录，创建了测试文件"

[5/10] ✅ Python执行                  1.8s
       Response: "执行结果: 5050"

[6/10] ✅ 网页抓取                    2.9s
       Response: "成功抓取 JSON 数据..."

[7/10] ❌ 知识库                      error
       Error: RAG database not initialized

[8/10] ✅ 画布管理                    2.5s
       Response: "已创建画布并添加节点"

[9/10] ✅ 安全状态                    1.1s
       Response: "沙箱状态: Running, 权限: 读写 /workspace..."

[10/10] ✅ 技能列表                   1.9s
        Response: "共 130 个 skills，分为 15 个类别..."

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## 下一步

1. ✅ 启动 UI 并运行自动测试
2. ✅ 查看测试结果
3. ✅ 如果有失败的测试，检查日志和配置
4. ✅ 手动测试特定功能验证

**现在请在 UI 中点击 "🧪 Auto Test" 按钮开始测试！**
