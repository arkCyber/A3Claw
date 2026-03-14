# OpenClaw+ 浏览器自动化工具集成指南

## 📋 概述

OpenClaw+ 支持多种浏览器自动化工具，用于实现网页操作、数据抓取和 AI Agent 的网页交互能力。

### 支持的工具

1. **Playwright** - 现代浏览器自动化框架（推荐）
2. **Firecrawl** - 智能网页内容提取（AI 友好）
3. **Jina Reader** - 快速网页转 Markdown
4. **Puppeteer** - Chrome DevTools Protocol 客户端
5. **Selenium** - 传统浏览器自动化（兼容性）

---

## 🎯 工具选择指南

### 场景 1: AI Agent 网页操作

**推荐**: Playwright + Firecrawl

- **Playwright**: 执行点击、填表、截图等操作
- **Firecrawl**: 将网页转换为干净的 Markdown，供 Qwen 2.5 模型理解

```javascript
// 示例：使用 Playwright 打开网页，Firecrawl 提取内容
const page = await browser.newPage();
await page.goto('https://example.com');
const markdown = await firecrawl.scrape(page.url());
```

### 场景 2: 数据抓取和爬虫

**推荐**: Playwright + Jina Reader

- **Playwright**: 处理动态加载的内容
- **Jina Reader**: 快速提取文本内容

```bash
# Jina Reader API 示例
curl https://r.jina.ai/https://example.com
```

### 场景 3: 桌面应用自动化

**推荐**: PyAutoGUI 或 Anthropic Computer Use

- **PyAutoGUI**: 本地鼠标/键盘模拟
- **Computer Use**: Claude 3.5 Sonnet 的屏幕操作能力

### 场景 4: 旧版浏览器兼容

**推荐**: Selenium

- 支持 IE、旧版 Firefox 等
- 企业内网环境

---

## 🚀 Playwright 集成

### 安装

```bash
# 安装 Playwright
cd vendor/openclaw
pnpm add -D playwright @playwright/test

# 安装浏览器
pnpm exec playwright install chromium
```

### 配置

在 `~/.config/openclaw-plus/config.toml` 中添加：

```toml
[browser]
enabled = true
engine = "playwright"
headless = false
viewport_width = 1280
viewport_height = 800
timeout_ms = 30000

[browser.playwright]
browser_type = "chromium"  # chromium, firefox, webkit
executable_path = ""  # 留空使用默认路径
user_data_dir = "~/.openclaw/browser-data"
```

### Rust 集成代码

创建 `crates/agent-executor/src/builtin_tools/playwright_backend.rs`:

```rust
//! Playwright 浏览器自动化后端
//! 
//! 通过 Node.js 子进程调用 Playwright API

use std::process::{Command, Stdio};
use serde_json::Value;

pub struct PlaywrightBackend {
    node_path: String,
    script_path: String,
}

impl PlaywrightBackend {
    pub fn new() -> Result<Self, String> {
        // 查找 Node.js
        let node_path = which::which("node")
            .map_err(|e| format!("Node.js not found: {}", e))?
            .to_string_lossy()
            .to_string();
        
        // Playwright 脚本路径
        let script_path = format!(
            "{}/vendor/openclaw/scripts/playwright-bridge.js",
            env!("CARGO_MANIFEST_DIR")
        );
        
        Ok(Self { node_path, script_path })
    }
    
    pub async fn screenshot(&self, url: &str, width: u32, height: u32) -> Result<String, String> {
        let args = serde_json::json!({
            "action": "screenshot",
            "url": url,
            "viewport": { "width": width, "height": height }
        });
        
        self.execute(&args).await
    }
    
    pub async fn navigate(&self, url: &str) -> Result<String, String> {
        let args = serde_json::json!({
            "action": "navigate",
            "url": url
        });
        
        self.execute(&args).await
    }
    
    pub async fn click(&self, selector: &str) -> Result<String, String> {
        let args = serde_json::json!({
            "action": "click",
            "selector": selector
        });
        
        self.execute(&args).await
    }
    
    pub async fn fill(&self, selector: &str, value: &str) -> Result<String, String> {
        let args = serde_json::json!({
            "action": "fill",
            "selector": selector,
            "value": value
        });
        
        self.execute(&args).await
    }
    
    async fn execute(&self, args: &Value) -> Result<String, String> {
        let output = Command::new(&self.node_path)
            .arg(&self.script_path)
            .arg(args.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Failed to execute Playwright: {}", e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Playwright error: {}", stderr));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }
}
```

### Node.js 桥接脚本

创建 `vendor/openclaw/scripts/playwright-bridge.js`:

```javascript
#!/usr/bin/env node
/**
 * Playwright 桥接脚本
 * 接收 JSON 参数，执行 Playwright 操作，返回结果
 */

const { chromium } = require('playwright');

async function main() {
  const args = JSON.parse(process.argv[2] || '{}');
  const { action, url, selector, value, viewport } = args;
  
  const browser = await chromium.launch({
    headless: process.env.HEADLESS !== 'false',
  });
  
  const context = await browser.newContext({
    viewport: viewport || { width: 1280, height: 800 },
  });
  
  const page = await context.newPage();
  
  try {
    switch (action) {
      case 'screenshot':
        await page.goto(url, { waitUntil: 'networkidle' });
        const screenshot = await page.screenshot({ 
          type: 'png',
          fullPage: false 
        });
        console.log(screenshot.toString('base64'));
        break;
        
      case 'navigate':
        await page.goto(url, { waitUntil: 'networkidle' });
        const title = await page.title();
        console.log(JSON.stringify({ 
          url: page.url(), 
          title 
        }));
        break;
        
      case 'click':
        await page.click(selector);
        console.log(JSON.stringify({ 
          success: true, 
          selector 
        }));
        break;
        
      case 'fill':
        await page.fill(selector, value);
        console.log(JSON.stringify({ 
          success: true, 
          selector, 
          value 
        }));
        break;
        
      default:
        throw new Error(`Unknown action: ${action}`);
    }
  } finally {
    await browser.close();
  }
}

main().catch(err => {
  console.error(err.message);
  process.exit(1);
});
```

---

## 🔥 Firecrawl 集成

### 安装

```bash
# 安装 Firecrawl SDK
cd vendor/openclaw
pnpm add @mendable/firecrawl-js
```

### 配置

```toml
[firecrawl]
enabled = true
api_key = ""  # 留空使用本地模式
endpoint = "http://localhost:3002"  # 本地 Firecrawl 服务
timeout_ms = 30000
```

### Rust 集成代码

创建 `crates/agent-executor/src/builtin_tools/firecrawl.rs`:

```rust
//! Firecrawl 网页内容提取
//! 
//! 将网页转换为 AI 友好的 Markdown 格式

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct FirecrawlRequest {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    formats: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct FirecrawlResponse {
    success: bool,
    data: Option<FirecrawlData>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FirecrawlData {
    markdown: String,
    html: Option<String>,
    metadata: Option<serde_json::Value>,
}

pub struct FirecrawlClient {
    client: Client,
    endpoint: String,
    api_key: Option<String>,
}

impl FirecrawlClient {
    pub fn new(endpoint: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            api_key,
        }
    }
    
    pub async fn scrape(&self, url: &str) -> Result<String, String> {
        let req = FirecrawlRequest {
            url: url.to_string(),
            formats: Some(vec!["markdown".to_string()]),
        };
        
        let mut request = self.client
            .post(format!("{}/v1/scrape", self.endpoint))
            .json(&req);
        
        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
        
        let resp = request
            .send()
            .await
            .map_err(|e| format!("Firecrawl request failed: {}", e))?;
        
        let data: FirecrawlResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Firecrawl response: {}", e))?;
        
        if !data.success {
            return Err(data.error.unwrap_or_else(|| "Unknown error".to_string()));
        }
        
        Ok(data.data
            .ok_or("No data in response")?
            .markdown)
    }
}
```

### Node.js 示例

创建 `vendor/openclaw/examples/firecrawl-example.js`:

```javascript
import Firecrawl from '@mendable/firecrawl-js';

const app = new Firecrawl({
  apiKey: process.env.FIRECRAWL_API_KEY || '',
});

async function scrapeWebpage(url) {
  try {
    const result = await app.scrapeUrl(url, {
      formats: ['markdown', 'html'],
    });
    
    console.log('=== Markdown 内容 ===');
    console.log(result.markdown);
    
    return result.markdown;
  } catch (error) {
    console.error('Firecrawl 错误:', error.message);
    throw error;
  }
}

// 示例使用
scrapeWebpage('https://docs.openclaw.ai/configuration')
  .then(markdown => {
    console.log(`\n提取了 ${markdown.length} 字符的 Markdown 内容`);
  });
```

---

## 🌐 Jina Reader 集成

### 简单集成（无需安装）

Jina Reader 是一个 HTTP API，无需安装：

```rust
//! Jina Reader 集成
//! 
//! 通过 HTTP API 快速将网页转换为 Markdown

use reqwest::Client;

pub struct JinaReader {
    client: Client,
}

impl JinaReader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    
    pub async fn read(&self, url: &str) -> Result<String, String> {
        let jina_url = format!("https://r.jina.ai/{}", url);
        
        let resp = self.client
            .get(&jina_url)
            .header("User-Agent", "Mozilla/5.0 (compatible; OpenClaw/1.0)")
            .send()
            .await
            .map_err(|e| format!("Jina Reader request failed: {}", e))?;
        
        if !resp.status().is_success() {
            return Err(format!("Jina Reader returned status: {}", resp.status()));
        }
        
        resp.text()
            .await
            .map_err(|e| format!("Failed to read Jina Reader response: {}", e))
    }
}
```

### 使用示例

```bash
# 命令行使用
curl https://r.jina.ai/https://docs.openclaw.ai/configuration

# 在 OpenClaw Agent 中使用
openclaw agent run --skill web.read --args '{"url": "https://example.com"}'
```

---

## 🧪 测试脚本

创建 `tests/test_browser_automation.sh`:

```bash
#!/usr/bin/env bash
# 浏览器自动化工具测试脚本

set -euo pipefail

echo "=== OpenClaw+ 浏览器自动化工具测试 ==="

# 测试 1: Playwright 安装
echo "1. 检查 Playwright 安装..."
if command -v node &> /dev/null; then
    echo "✓ Node.js 已安装"
    if [ -f "vendor/openclaw/node_modules/playwright/index.js" ]; then
        echo "✓ Playwright 已安装"
    else
        echo "✗ Playwright 未安装，运行: cd vendor/openclaw && pnpm add -D playwright"
    fi
else
    echo "✗ Node.js 未安装"
fi

# 测试 2: Jina Reader API
echo ""
echo "2. 测试 Jina Reader API..."
JINA_RESULT=$(curl -s https://r.jina.ai/https://example.com | head -c 100)
if [ -n "$JINA_RESULT" ]; then
    echo "✓ Jina Reader API 正常"
    echo "  预览: ${JINA_RESULT}..."
else
    echo "✗ Jina Reader API 失败"
fi

# 测试 3: Firecrawl 本地服务
echo ""
echo "3. 检查 Firecrawl 服务..."
if curl -s http://localhost:3002/health &> /dev/null; then
    echo "✓ Firecrawl 本地服务运行中"
else
    echo "⚠ Firecrawl 本地服务未运行"
    echo "  启动方式: docker run -p 3002:3002 mendableai/firecrawl"
fi

# 测试 4: Rust 浏览器模块编译
echo ""
echo "4. 测试 Rust 浏览器模块..."
if cargo check -p openclaw-agent-executor 2>&1 | grep -q "Finished"; then
    echo "✓ agent-executor 编译通过"
else
    echo "✗ agent-executor 编译失败"
fi

echo ""
echo "=== 测试完成 ==="
```

---

## 📚 完整示例

### 示例 1: AI Agent 网页搜索

```rust
// 使用 Playwright + Firecrawl 实现智能网页搜索

use crate::builtin_tools::{PlaywrightBackend, FirecrawlClient};

pub async fn ai_web_search(query: &str) -> Result<String, String> {
    // 1. 使用 Playwright 打开搜索引擎
    let playwright = PlaywrightBackend::new()?;
    playwright.navigate("https://www.google.com/search?q={query}").await?;
    
    // 2. 获取第一个搜索结果链接
    let first_link = playwright.get_attribute("a.result-link", "href").await?;
    
    // 3. 使用 Firecrawl 提取内容
    let firecrawl = FirecrawlClient::new(
        "http://localhost:3002".to_string(),
        None
    );
    let markdown = firecrawl.scrape(&first_link).await?;
    
    // 4. 返回 AI 友好的 Markdown
    Ok(markdown)
}
```

### 示例 2: 表单自动填写

```javascript
// vendor/openclaw/examples/form-automation.js

const { chromium } = require('playwright');

async function fillForm(url, formData) {
  const browser = await chromium.launch({ headless: false });
  const page = await browser.newPage();
  
  await page.goto(url);
  
  // 填写表单
  for (const [selector, value] of Object.entries(formData)) {
    await page.fill(selector, value);
  }
  
  // 提交
  await page.click('button[type="submit"]');
  
  // 等待结果
  await page.waitForNavigation();
  
  const result = await page.title();
  await browser.close();
  
  return result;
}

// 使用示例
fillForm('https://example.com/contact', {
  '#name': 'OpenClaw Agent',
  '#email': 'agent@openclaw.ai',
  '#message': 'Hello from AI!',
});
```

---

## 🔒 安全注意事项

1. **SSRF 防护**: 所有 URL 都经过 OpenClaw 的 SSRF 策略检查
2. **沙箱隔离**: 浏览器运行在独立的用户数据目录
3. **权限控制**: 需要用户确认才能执行敏感操作
4. **日志审计**: 所有浏览器操作都记录在审计日志中

---

## 📖 相关文档

- [OpenClaw Browser 文档](vendor/openclaw/docs/tools/browser.md)
- [Playwright 官方文档](https://playwright.dev)
- [Firecrawl 文档](https://docs.firecrawl.dev)
- [Jina Reader API](https://jina.ai/reader)

---

**最后更新**: 2026-03-02  
**维护者**: OpenClaw+ Team  
**许可**: MIT
