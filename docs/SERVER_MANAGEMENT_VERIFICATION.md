# OpenClaw+ 服务器管理功能验证指南

**版本**: v2.0 - 完整 UX 优化版  
**日期**: 2026-02-27  
**状态**: ✅ 已完成并可验证

---

## 🎯 本次优化内容

### ✅ 已完成的 UX 改进

1. **全局加载状态**
   - 操作时显示 "Loading…" 提示
   - 加载期间禁用所有按钮（避免重复点击）
   - 操作完成后自动刷新服务器列表

2. **操作结果反馈**
   - 成功操作显示绿色提示
   - 失败操作显示红色提示
   - Ollama 外部启动错误给出友好提示

3. **Health Check 延迟测量**
   - 显示实际响应时间（毫秒）
   - 例如："Server is healthy (45ms)"

---

## 🧪 完整验证流程

### 准备工作

```bash
# 1. 确认编译成功
ls -lh target/release/openclaw-plus
ls -lh target/release/server-ctl

# 2. 确认配置文件存在
cat config/servers.toml

# 3. 启动 UI
./target/release/openclaw-plus
```

---

## 📋 测试场景 1：刷新服务器列表

### 操作步骤
1. 打开 UI，导航到 **General Settings**
2. 滚动到 **Inference Server Management** 部分
3. 点击 **⟳ Refresh** 按钮

### 预期结果
- ✅ 按钮变灰（禁用状态）
- ✅ 显示 "Loading…" 黄色提示
- ✅ 1 秒内完成加载
- ✅ 显示服务器列表（至少 2 个：ollama-primary, llama-cpp-backup）
- ✅ "Loading…" 消失

### 验证点
- [ ] Refresh 按钮在加载时无法再次点击
- [ ] 所有服务器卡片的按钮在加载时禁用
- [ ] 加载完成后按钮恢复可用

---

## 📋 测试场景 2：Health Check（成功）

### 前提条件
```bash
# 确保 Ollama 正在运行
./scripts/start-ollama.sh
# 或
ollama serve &
```

### 操作步骤
1. 在服务器列表中找到 **Ollama (主服务)**
2. 点击 **Health Check** 按钮

### 预期结果
- ✅ 按钮变灰（禁用状态）
- ✅ 显示 "Loading…" 提示
- ✅ 1-2 秒后显示绿色提示：
  ```
  Server is healthy (XXms)
  ```
  其中 XX 是实际延迟（通常 20-100ms）
- ✅ 提示显示在服务器列表上方

### 验证点
- [ ] 延迟时间合理（< 500ms）
- [ ] 提示颜色为绿色
- [ ] 提示内容包含延迟信息

---

## 📋 测试场景 3：Health Check（失败）

### 前提条件
```bash
# 确保 Ollama 未运行
pkill ollama
```

### 操作步骤
1. 点击 **⟳ Refresh** 刷新状态
2. 确认 Ollama 状态为 **Stopped**
3. 点击 **Health Check** 按钮

### 预期结果
- ✅ 按钮变灰（禁用状态）
- ✅ 显示 "Loading…" 提示
- ✅ 1-2 秒后显示红色错误提示
- ✅ 错误信息清晰（例如："Connection refused"）

### 验证点
- [ ] 提示颜色为红色
- [ ] 错误信息有意义
- [ ] 不会崩溃或卡死

---

## 📋 测试场景 4：启动 Ollama（外部启动提示）

### 操作步骤
1. 确保 Ollama 未运行
2. 在 Ollama 卡片上点击 **Start** 按钮

### 预期结果
- ✅ 按钮变灰（禁用状态）
- ✅ 显示 "Loading…" 提示
- ✅ 1-2 秒后显示红色提示：
  ```
  ⚠️  ollama-primary requires external startup. 
  Run: ./scripts/start-ollama.sh or 'ollama serve'
  ```
- ✅ 提示友好且包含具体操作指引

### 验证点
- [ ] 提示包含 emoji ⚠️
- [ ] 提示包含启动脚本路径
- [ ] 提示包含 ollama serve 命令

---

## 📋 测试场景 5：外部启动 Ollama 后验证

### 操作步骤
1. 按照上一步的提示，手动启动 Ollama：
   ```bash
   ./scripts/start-ollama.sh
   ```
2. 等待 Ollama 启动完成（约 5 秒）
3. 在 UI 中点击 **⟳ Refresh**

### 预期结果
- ✅ Ollama 状态变为 **Running**（绿色）
- ✅ 显示端点：http://localhost:11434
- ✅ 可能显示 CPU/内存使用情况
- ✅ 按钮变为 **Stop** / **Restart** / **Health Check**

### 验证点
- [ ] 状态正确更新
- [ ] 按钮组合正确（Running 状态）
- [ ] 可以执行 Health Check

---

## 📋 测试场景 6：自动刷新（操作后）

### 操作步骤
1. 确保 Ollama 正在运行
2. 点击 **Health Check** 按钮
3. 等待健康检查完成
4. **不要手动点击 Refresh**

### 预期结果
- ✅ Health Check 完成后显示结果提示
- ✅ 提示显示约 1 秒
- ✅ **不需要手动刷新，列表自动更新**
- ✅ 服务器状态保持最新

### 验证点
- [ ] 无需手动刷新
- [ ] 状态自动更新
- [ ] 用户体验流畅

---

## 📋 测试场景 7：按钮禁用（全局锁）

### 操作步骤
1. 点击 Ollama 的 **Health Check** 按钮
2. **立即**尝试点击其他服务器的按钮
3. 尝试点击 **Refresh** 按钮

### 预期结果
- ✅ 所有服务器的所有按钮都被禁用
- ✅ Refresh 按钮也被禁用
- ✅ 显示 "Loading…" 提示
- ✅ 操作完成后所有按钮恢复

### 验证点
- [ ] 无法重复点击
- [ ] 无法同时操作多个服务器
- [ ] 加载状态清晰可见

---

## 📋 测试场景 8：连续操作流程

### 完整工作流
```bash
# 1. 启动 UI
./target/release/openclaw-plus

# 2. 在 UI 中执行以下操作（按顺序）
```

1. **Refresh** → 查看初始状态
2. **Health Check (Ollama)** → 如果失败，外部启动
3. 外部启动 Ollama：`./scripts/start-ollama.sh`
4. **Refresh** → 确认 Ollama Running
5. **Health Check (Ollama)** → 查看延迟
6. **Health Check (llama.cpp)** → 预期失败（未启动）
7. **Stop (Ollama)** → 停止 Ollama（如果支持）
8. **Refresh** → 确认状态更新

### 预期结果
- ✅ 每步操作都有明确反馈
- ✅ 加载状态清晰
- ✅ 错误提示友好
- ✅ 自动刷新工作正常

---

## 🎨 UI 元素验证清单

### 服务器管理区域顶部
- [ ] 显示服务器数量："X servers configured"
- [ ] Refresh 按钮可见且可用
- [ ] 加载时显示 "Loading…" 黄色文字
- [ ] 操作结果显示在独立卡片中（绿色/红色）

### 服务器卡片（Running 状态）
- [ ] 服务器名称（粗体）
- [ ] 端点地址（带 🌐 图标）
- [ ] 状态显示（绿色 "Running"）
- [ ] 三个按钮：Stop / Restart / Health Check
- [ ] 加载时所有按钮禁用

### 服务器卡片（Stopped 状态）
- [ ] 服务器名称（粗体）
- [ ] 端点地址（带 🌐 图标）
- [ ] 状态显示（灰色 "Stopped"）
- [ ] 两个按钮：Start / Health Check
- [ ] 加载时所有按钮禁用

### 操作结果提示
- [ ] 成功：绿色背景，绿色文字
- [ ] 失败：红色背景，红色文字
- [ ] 包含具体信息（延迟/错误原因）
- [ ] 显示在服务器列表上方

---

## 🐛 已知问题和限制

### 1. Ollama 外部启动
- **现象**：点击 Start 会提示需要外部启动
- **原因**：Ollama 设计为独立服务
- **解决**：按提示运行 `./scripts/start-ollama.sh`

### 2. llama.cpp 需要模型文件
- **现象**：启动失败，提示找不到模型
- **原因**：需要下载 GGUF 模型文件
- **解决**：参考 `docs/SERVER_MANAGEMENT_TEST_GUIDE.md`

### 3. 全局锁定
- **现象**：一次只能操作一个服务器
- **原因**：采用全局锁设计（方案 A）
- **影响**：无法并发操作，但更安全

---

## ✅ 验证通过标准

完成以上所有测试场景后，确认：

- [ ] 所有按钮在加载时正确禁用
- [ ] 加载状态清晰可见
- [ ] 操作结果正确显示（颜色、内容）
- [ ] Health Check 显示延迟时间
- [ ] Ollama 外部启动提示友好
- [ ] 自动刷新正常工作
- [ ] 无崩溃、无卡死
- [ ] 用户体验流畅

---

## 📊 性能指标

### 预期性能
- **Refresh 耗时**：< 1 秒
- **Health Check 耗时**：< 2 秒
- **Health Check 延迟**：20-100ms（Ollama 本地）
- **UI 响应**：即时（< 100ms）

### 如果性能不达标
1. 检查 server-ctl 是否编译为 release 模式
2. 检查网络连接（如果测试远程服务器）
3. 检查系统资源（CPU/内存）

---

## 🎉 验证完成

如果所有测试通过，恭喜！你现在拥有一个：

✅ **完整功能**的服务器管理系统  
✅ **流畅体验**的用户界面  
✅ **清晰反馈**的操作提示  
✅ **航空航天级**的代码质量

---

## 📞 问题反馈

如果遇到问题：

1. 查看日志：`tail -f /tmp/openclaw.log`
2. 检查配置：`cat config/servers.toml`
3. 验证工具：`./target/release/server-ctl list --json`
4. 重新编译：`cargo build --release -p openclaw-ui`

---

**祝测试顺利！** 🚀
