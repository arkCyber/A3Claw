# Assistant 工具系统完整指南

**版本**: v2.0  
**日期**: 2026-03-08  
**功能**: 配置文件操作 + 系统自动启动 + 定时健康监控

---

## 🎯 功能概述

Assistant 现在具备**完整的系统管理能力**：

### ✅ 已实现功能

1. **配置文件操作** - 读取、修改、保存配置
2. **系统自动启动** - 一键启动所有组件
3. **定时健康监控** - 自动检测系统状态
4. **诊断和修复** - 检查并修复常见问题
5. **操作指导** - 详细的使用教程

---

## 🔧 完整工具列表

| 工具名称 | 功能 | 触发关键词 | 状态 |
|---------|------|-----------|------|
| **update_config** | 修改配置文件 | "修改配置"、"更新配置" | ✅ |
| **auto_start_system** | 自动启动系统 | "自动启动"、"启动系统" | ✅ |
| **enable_health_monitor** | 启用健康监控 | "启用监控"、"开启监控" | ✅ |
| **disable_health_monitor** | 禁用健康监控 | "禁用监控"、"关闭监控" | ✅ |
| **check_ollama_health** | 检查 Ollama 服务 | "检查 Ollama"、"Ollama 状态" | ✅ |
| **start_ollama_service** | 启动 Ollama | "启动 Ollama" | ✅ |
| **check_config** | 检查配置文件 | "检查配置" | ✅ |
| **get_system_status** | 系统整体状态 | "系统状态"、"健康检查" | ✅ |
| **provide_guide** | 操作指南 | "如何"、"指南" | ✅ |

---

## 📝 使用示例

### 示例 1: 修改配置文件

**用户输入**:
```
修改配置，将 Ollama 端点设置为 http://localhost:11434
```

**Assistant 响应**:
```
🔧 正在执行: update_config...

✅ 配置已更新
路径: ~/Library/Application Support/openclaw-plus/config.toml
配置项: ollama_endpoint = http://localhost:11434

配置文件已成功更新。新的端点将在下次启动时生效。
```

---

### 示例 2: 自动启动系统

**用户输入**:
```
自动启动系统
```

**Assistant 响应**:
```
🔧 正在执行: auto_start_system...

🚀 正在启动系统组件...

【1/2】Ollama 服务
⏳ 正在启动...
✅ Ollama 服务已成功启动

【2/2】配置文件
✅ 配置正常

━━━━━━━━━━━━━━━━━━━━━━
✅ 系统启动流程完成！

建议操作：
1. 前往 AI Chat 页面测试对话
2. 检查模型列表是否显示
3. 如有问题，查看详细日志

系统已完全启动，所有组件运行正常！
```

---

### 示例 3: 启用健康监控

**用户输入**:
```
启用健康监控，每 5 分钟检查一次
```

**Assistant 响应**:
```
🔧 正在执行: enable_health_monitor...

✅ 健康监控已启用

检测间隔: 5 分钟
监控内容:
- Ollama 服务状态
- 配置文件有效性
- UI 运行状态

配置文件: ~/Library/Application Support/openclaw-plus/health_monitor.toml

注意: 监控将在下次 UI 启动时生效

健康监控已配置完成。系统将定期检查所有组件状态，
如发现问题会自动尝试修复或通知您。
```

---

### 示例 4: 禁用健康监控

**用户输入**:
```
禁用健康监控
```

**Assistant 响应**:
```
🔧 正在执行: disable_health_monitor...

✅ 健康监控已禁用

配置文件已删除: ~/Library/Application Support/openclaw-plus/health_monitor.toml

监控将在下次 UI 启动时停止

健康监控已成功禁用。
```

---

## 🎨 完整工作流程

```
用户: "自动启动系统"
  ↓
关键词检测 → 触发 auto_start_system
  ↓
显示: "🔧 正在执行: auto_start_system..."
  ↓
1. 检查 Ollama 服务
   - 如未运行 → 启动服务
   - 如已运行 → 跳过
  ↓
2. 验证配置文件
   - 检查格式
   - 验证必要配置项
  ↓
显示完整启动报告
  ↓
AI 分析结果 → 提供后续建议
```

---

## 🔍 关键词触发规则

### update_config
触发条件：包含 "配置" 或 "config" **且** 包含以下之一：
- 中文: "修改"、"更新"、"设置"
- 英文: "update"、"modify"、"change"

### auto_start_system
触发条件：包含以下之一：
- 中文: "自动启动"、"启动系统"
- 英文: "auto start"、"start system"

### enable_health_monitor
触发条件：包含 "启用/enable/开启/打开" **且** 包含 "监控/monitor/定时检测/health check"

### disable_health_monitor
触发条件：包含 "禁用/disable/关闭/停止" **且** 包含 "监控/monitor/定时检测/health check"

---

## 📚 配置文件说明

### 主配置文件
**路径**: `~/Library/Application Support/openclaw-plus/config.toml`

**示例内容**:
```toml
ollama_endpoint = "http://localhost:11434"
ollama_model = "qwen2.5:0.5b"
ui_theme = "dark"
auto_start = true
```

### 健康监控配置
**路径**: `~/Library/Application Support/openclaw-plus/health_monitor.toml`

**示例内容**:
```toml
enabled = true
interval_minutes = 5.0
last_check = 0
```

---

## 🧪 测试用例

### 测试 1: 配置文件修改

**输入**: "修改配置，设置 ollama_model 为 llama3.2"

**预期**:
1. 显示 "🔧 正在执行: update_config..."
2. 读取现有配置
3. 更新 ollama_model 值
4. 保存配置文件
5. 显示成功消息
6. AI 提供后续建议

---

### 测试 2: 系统自动启动

**输入**: "启动系统"

**预期**:
1. 显示 "🔧 正在执行: auto_start_system..."
2. 检查 Ollama 服务状态
3. 如未运行则启动
4. 验证配置文件
5. 显示完整启动报告
6. AI 提供使用建议

---

### 测试 3: 启用监控

**输入**: "启用健康监控"

**预期**:
1. 显示 "🔧 正在执行: enable_health_monitor..."
2. 创建监控配置文件
3. 设置默认间隔（5分钟）
4. 显示监控详情
5. AI 说明监控内容

---

### 测试 4: 禁用监控

**输入**: "关闭监控"

**预期**:
1. 显示 "🔧 正在执行: disable_health_monitor..."
2. 删除监控配置文件
3. 显示确认消息
4. AI 确认监控已停止

---

## 🛠️ 故障排除

### 问题 1: 配置文件修改失败

**症状**: 显示 "无法写入配置文件"

**原因**: 
- 配置目录不存在
- 文件权限问题
- 磁盘空间不足

**解决**:
```bash
# 检查目录
ls -la ~/Library/Application\ Support/openclaw-plus/

# 创建目录
mkdir -p ~/Library/Application\ Support/openclaw-plus/

# 检查权限
chmod 755 ~/Library/Application\ Support/openclaw-plus/
```

---

### 问题 2: 自动启动失败

**症状**: Ollama 服务启动失败

**原因**:
- Ollama 未安装
- 端口被占用
- 权限不足

**解决**:
```bash
# 检查 Ollama 安装
which ollama

# 检查端口占用
lsof -i :11434

# 手动启动
ollama serve
```

---

### 问题 3: 健康监控不生效

**症状**: 启用监控后没有定期检查

**原因**: 监控在下次 UI 启动时才生效

**解决**:
1. 重启 UI 应用
2. 检查监控配置文件是否存在
3. 查看日志确认监控运行

---

## 🔐 安全考虑

### 工具权限

| 工具 | 权限级别 | 风险 | 建议 |
|------|---------|------|------|
| **update_config** | 写入配置 | 中 | 验证配置值 |
| **auto_start_system** | 启动进程 | 中 | 仅启动已知服务 |
| **enable_health_monitor** | 写入配置 | 低 | 安全 |
| **disable_health_monitor** | 删除配置 | 低 | 安全 |
| **check_ollama_health** | 只读 | 低 | 安全 |
| **start_ollama_service** | 启动进程 | 中 | 仅启动 Ollama |
| **check_config** | 只读 | 低 | 安全 |
| **get_system_status** | 只读 | 低 | 安全 |
| **provide_guide** | 只读 | 低 | 安全 |

---

## 📊 性能指标

| 指标 | 典型值 | 说明 |
|------|--------|------|
| 工具检测延迟 | < 1ms | 关键词匹配 |
| 配置文件读取 | 5-20ms | 取决于文件大小 |
| 配置文件写入 | 10-50ms | 包含格式化 |
| Ollama 健康检查 | 100ms-2s | 网络请求 |
| Ollama 服务启动 | 2-5s | 进程启动 |
| 系统自动启动 | 3-10s | 多个步骤 |
| 监控配置操作 | 10-30ms | 文件操作 |

---

## 🚀 高级功能

### 1. 批量配置更新

**输入**: "修改配置，设置 endpoint 为 localhost:11434，model 为 llama3.2"

**实现**: Assistant 可以识别多个配置项并逐一更新

---

### 2. 条件启动

**输入**: "如果 Ollama 未运行则启动"

**实现**: auto_start_system 会自动检查状态并决定是否启动

---

### 3. 监控报告

**输入**: "显示最近的健康检查结果"

**实现**: 读取监控配置中的 last_check 时间戳

---

## 📝 最佳实践

### 1. 配置管理

✅ **推荐**:
- 使用 Assistant 修改配置
- 修改后验证配置有效性
- 保留配置备份

❌ **避免**:
- 手动编辑配置文件（容易出错）
- 修改未知配置项
- 删除必要配置

---

### 2. 系统启动

✅ **推荐**:
- 使用 auto_start_system 一键启动
- 启动后验证各组件状态
- 查看启动日志

❌ **避免**:
- 重复启动已运行的服务
- 忽略启动错误
- 跳过配置验证

---

### 3. 健康监控

✅ **推荐**:
- 设置合理的检查间隔（5-10分钟）
- 定期查看监控报告
- 及时处理告警

❌ **避免**:
- 间隔过短（< 1分钟，浪费资源）
- 间隔过长（> 30分钟，延迟发现问题）
- 忽略监控告警

---

## 🎉 总结

Assistant 工具系统现在提供：

✅ **配置文件操作** - 安全、便捷的配置管理  
✅ **系统自动启动** - 一键启动所有组件  
✅ **定时健康监控** - 主动发现和修复问题  
✅ **智能诊断** - 快速定位故障原因  
✅ **操作指导** - 详细的使用教程  

**Assistant 不仅能对话，还能真正管理和维护您的系统！** 🚀

---

## 📞 获取帮助

如需帮助，请在 Assistant 中输入：

- "如何使用 Assistant" - 获取使用指南
- "系统状态" - 查看整体健康状况
- "故障排除指南" - 获取常见问题解决方案
- "如何配置 AI Chat" - AI Chat 配置教程
- "如何安装 Ollama" - Ollama 安装指南
