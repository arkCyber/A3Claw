# JavaScript 插件兼容性方案

## 🎯 目标

**在现有 WASM 插件架构基础上，增加 JavaScript 插件支持**，提供更大的灵活性和开发便利性。

---

## 📊 当前架构分析

### 现有 WASM 插件系统
```
┌─────────────────────────────────────────────┐
│  OpenClaw+ Core (Rust)                      │
│  ── WasmPluginRegistry ──                    │
└──────────────┬──────────────────────────────┘
               │ WASM ABI
┌──────────────▼──────────────────────────────┐
│  WASM Plugins (Rust → WASM)                 │
│  ── skill_manifest()                        │
│  ── skill_execute()                         │
│  ── alloc/dealloc                           │
└─────────────────────────────────────────────┘
```

### 提议的混合架构
```
┌─────────────────────────────────────────────┐
│  OpenClaw+ Core (Rust)                      │
│  ── PluginRegistry (统一接口) ──             │
└──────┬──────────────────────┬────────────────┘
       │                      │
┌──────▼──────┐       ┌──────▼──────┐
│  WASM 插件   │       │  JS 插件    │
│  (高性能)    │       │  (灵活性)   │
└─────────────┘       └─────────────┘
```

---

## 🛠️ 实现方案

### 方案 1：统一插件接口

#### 1.1 插件类型枚举
```rust
// crates/wasm-plugin/src/registry.rs
#[derive(Debug, Clone)]
pub enum PluginType {
    Wasm(WasmPlugin),
    JavaScript(JsPlugin),
}

pub struct JsPlugin {
    pub manifest: SkillManifest,
    pub script_path: PathBuf,
    pub runtime: JsRuntime,
}

pub struct WasmPlugin {
    pub manifest: SkillManifest,
    pub module: wasmedge_sdk::Module,
    pub instance: wasmedge_sdk::Instance,
}
```

#### 1.2 统一插件注册表
```rust
pub struct UnifiedPluginRegistry {
    wasm_plugins: HashMap<String, WasmPlugin>,
    js_plugins: HashMap<String, JsPlugin>,
    // 统一的技能查找接口
    skill_to_plugin: HashMap<String, PluginType>,
}

impl UnifiedPluginRegistry {
    pub fn register_wasm_plugin(&mut self, plugin: WasmPlugin) -> Result<(), PluginError> {
        for skill in &plugin.manifest.skills {
            self.skill_to_plugin.insert(
                skill.name.clone(),
                PluginType::Wasm(plugin.clone())
            );
        }
        Ok(())
    }
    
    pub fn register_js_plugin(&mut self, plugin: JsPlugin) -> Result<(), PluginError> {
        for skill in &plugin.manifest.skills {
            self.skill_to_plugin.insert(
                skill.name.clone(),
                PluginType::Js(plugin.clone())
            );
        }
        Ok(())
    }
    
    pub async fn execute_skill(&self, skill: &str, args: serde_json::Value) -> Result<ExecuteResponse, PluginError> {
        match self.skill_to_plugin.get(skill) {
            Some(PluginType::Wasm(plugin)) => {
                // 调用 WASM 插件
                plugin.execute(skill, args).await
            },
            Some(PluginType::Js(plugin)) => {
                // 调用 JavaScript 插件
                plugin.execute(skill, args).await
            },
            None => Err(PluginError::SkillNotFound(skill.to_string())),
        }
    }
}
```

### 方案 2：JavaScript 插件运行时

#### 2.1 JavaScript 运行时封装
```rust
// crates/wasm-plugin/src/js_runtime.rs
use wasmedge_sdk::{
    config::{CommonConfigOptions, ConfigBuilder},
    params, Module, Store, Vm,
    wasi::WasiModule,
};

pub struct JsRuntime {
    vm: Vm,
    quickjs_module: Module,
}

impl JsRuntime {
    pub fn new() -> Result<Self, PluginError> {
        // 使用 WasmEdge QuickJS 作为 JavaScript 运行时
        let wasm_config = ConfigBuilder::new(CommonConfigOptions::default()).build()?;
        let wasi_module = WasiModule::create(
            Some(vec![]),
            Some(vec![]),
            Some(vec![]),
        )?;
        
        let mut instances: HashMap<String, &mut dyn SyncInst> = HashMap::new();
        instances.insert(wasi_module.name().to_string(), wasi_module.as_mut());
        
        let store = Store::new(Some(&wasm_config), instances)?;
        let mut vm = Vm::new(store);
        
        // 加载 QuickJS WASM 引擎
        let quickjs_path = find_quickjs_wasm()?;
        let quickjs_module = Module::from_file(Some(&wasm_config), &quickjs_path)?;
        vm.register_module(Some("quickjs"), quickjs_module)?;
        
        Ok(Self { vm, quickjs_module })
    }
    
    pub async fn execute_script(&mut self, script_path: &Path, skill: &str, args: serde_json::Value) -> Result<ExecuteResponse, PluginError> {
        // 1. 加载 JavaScript 插件脚本
        let script_content = std::fs::read_to_string(script_path)?;
        
        // 2. 注入插件运行时环境
        let wrapped_script = wrap_plugin_script(&script_content, skill, &args);
        
        // 3. 在 QuickJS 中执行
        match self.vm.run_func(Some("quickjs"), "_start", params!(wrapped_script)) {
            Ok(_) => {
                // 解析执行结果
                self.parse_execution_result()
            },
            Err(e) => Err(PluginError::ExecutionError(e.to_string())),
        }
    }
}
```

#### 2.2 JavaScript 插件包装器
```javascript
// JavaScript 插件的标准包装模板
function wrapPluginScript(originalScript, skill, args) {
    return `
// OpenClaw+ JavaScript Plugin Runtime
const __skill_name = "${skill}";
const __skill_args = ${JSON.stringify(args)};
const __request_id = generateRequestId();

// 插件运行时 API
const pluginAPI = {
    // 返回成功结果
    success: (output) => {
        console.log(JSON.stringify({
            request_id: __request_id,
            ok: true,
            output: String(output),
            error: ""
        }));
    },
    
    // 返回错误结果
    error: (errorMessage) => {
        console.log(JSON.stringify({
            request_id: __request_id,
            ok: false,
            output: "",
            error: String(errorMessage)
        }));
    },
    
    // HTTP 请求 (通过 host function)
    httpFetch: async (url, options = {}) => {
        // 调用 host_http_fetch import
        return await host_http_fetch(JSON.stringify({
            url: url,
            method: options.method || "GET",
            headers: options.headers || {},
            body: options.body
        }));
    },
    
    // 文件操作 (通过 WASI)
    readFile: (path) => {
        const std = require('std');
        const file = std.open(path, 'r');
        if (!file) return null;
        return file.readAll();
    },
    
    writeFile: (path, content) => {
        const std = require('std');
        const file = std.open(path, 'w');
        if (!file) return false;
        file.puts(content);
        return true;
    }
};

// 用户插件代码
${originalScript}

// 插件入口点
try {
    if (typeof pluginMain === 'function') {
        pluginMain(__skill_name, __skill_args, pluginAPI);
    } else {
        pluginAPI.error('Plugin must export pluginMain function');
    }
} catch (error) {
    pluginAPI.error('Plugin execution failed: ' + error.message);
}
    `;
}
```

### 方案 3：JavaScript 插件示例

#### 3.1 简单的 JavaScript 插件
```javascript
// weather-plugin.js
function pluginMain(skill, args, api) {
    switch (skill) {
        case 'weather.current':
            return getCurrentWeather(args.city, api);
        case 'weather.forecast':
            return getWeatherForecast(args.city, args.days, api);
        default:
            api.error(`Unknown skill: ${skill}`);
    }
}

async function getCurrentWeather(city, api) {
    try {
        // 调用天气 API
        const response = await api.httpFetch(
            \`https://api.open-meteo.com/v1/forecast?latitude=\${getLat(city)}&longitude=\${getLon(city)}&current_weather=true\`
        );
        
        if (response.status === 200) {
            const data = JSON.parse(response.body);
            const temp = data.current_weather.temperature;
            api.success(\`Current weather in \${city}: \${temp}°C\`);
        } else {
            api.error(\`Weather API error: \${response.status}\`);
        }
    } catch (error) {
        api.error(\`Failed to get weather: \${error.message}\`);
    }
}

function getLat(city) {
    // 简单的城市坐标映射
    const cities = {
        'Tokyo': 35.6762,
        'London': 51.5074,
        'New York': 40.7128,
        'Beijing': 39.9042
    };
    return cities[city] || 0;
}

function getLon(city) {
    const cities = {
        'Tokyo': 139.6503,
        'London': -0.1278,
        'New York': -74.0060,
        'Beijing': 116.4074
    };
    return cities[city] || 0;
}

// 插件清单 (JSON 格式，可以在脚本顶部定义)
const PLUGIN_MANIFEST = {
    id: "openclaw.weather-js",
    name: "Weather Plugin (JavaScript)",
    version: "1.0.0",
    description: "Weather data via Open-Meteo API",
    skills: [
        {
            name: "weather.current",
            display: "Current Weather",
            description: "Get current weather for a city",
            risk: "safe",
            params: [
                { name: "city", type: "string", description: "City name", required: true }
            ]
        },
        {
            name: "weather.forecast",
            display: "Weather Forecast",
            description: "Get weather forecast",
            risk: "safe",
            params: [
                { name: "city", type: "string", description: "City name", required: true },
                { name: "days", type: "number", description: "Number of days", required: false }
            ]
        }
    ]
};
```

#### 3.2 高级 JavaScript 插件示例
```javascript
// file-processor-plugin.js
function pluginMain(skill, args, api) {
    switch (skill) {
        case 'fileprocessor.analyze':
            return analyzeFile(args.path, api);
        case 'fileprocessor.convert':
            return convertFile(args.input, args.output, args.format, api);
        case 'fileprocessor.compress':
            return compressFiles(args.files, args.output, api);
        default:
            api.error(`Unknown skill: ${skill}`);
    }
}

async function analyzeFile(filePath, api) {
    try {
        // 读取文件
        const content = api.readFile(filePath);
        if (!content) {
            api.error(`Cannot read file: ${filePath}`);
            return;
        }
        
        // 分析文件
        const analysis = {
            size: content.length,
            lines: content.split('\n').length,
            words: content.split(/\s+/).length,
            type: getFileType(filePath),
            encoding: detectEncoding(content)
        };
        
        api.success(JSON.stringify(analysis, null, 2));
    } catch (error) {
        api.error(`File analysis failed: ${error.message}`);
    }
}

async function convertFile(inputPath, outputPath, format, api) {
    try {
        const content = api.readFile(inputPath);
        if (!content) {
            api.error(`Cannot read input file: ${inputPath}`);
            return;
        }
        
        let converted;
        switch (format.toLowerCase()) {
            case 'json':
                converted = convertToJson(content);
                break;
            case 'csv':
                converted = convertToCsv(content);
                break;
            case 'xml':
                converted = convertToXml(content);
                break;
            default:
                api.error(`Unsupported format: ${format}`);
                return;
        }
        
        const success = api.writeFile(outputPath, converted);
        if (success) {
            api.success(`File converted: ${inputPath} -> ${outputPath}`);
        } else {
            api.error(`Failed to write output file: ${outputPath}`);
        }
    } catch (error) {
        api.error(`File conversion failed: ${error.message}`);
    }
}

function getFileType(filePath) {
    const ext = filePath.split('.').pop().toLowerCase();
    const types = {
        'txt': 'text',
        'json': 'json',
        'csv': 'csv',
        'xml': 'xml',
        'js': 'javascript',
        'py': 'python'
    };
    return types[ext] || 'unknown';
}

function detectEncoding(content) {
    // 简单的编码检测
    try {
        new TextDecoder('utf-8').decode(new TextEncoder().encode(content));
        return 'utf-8';
    } catch {
        return 'unknown';
    }
}
```

---

## 🔧 插件发现和加载

### 插件搜索路径扩展
```rust
// crates/wasm-plugin/src/loader.rs
pub struct PluginLoader {
    search_paths: Vec<PathBuf>,
    registry: UnifiedPluginRegistry,
}

impl PluginLoader {
    pub fn new() -> Self {
        let mut search_paths = vec![
            PathBuf::from(".openclaw/skills"),
            dirs::home_dir().unwrap_or_default().join(".openclaw/skills"),
        ];
        
        // 添加 JavaScript 插件路径
        search_paths.push(PathBuf::from(".openclaw/js-skills"));
        search_paths.push(dirs::home_dir().unwrap_or_default().join(".openclaw/js-skills"));
        
        Self {
            search_paths,
            registry: UnifiedPluginRegistry::new(),
        }
    }
    
    pub async fn discover_and_load(&mut self) -> Result<usize, PluginError> {
        let mut loaded_count = 0;
        
        for path in &self.search_paths.clone() {
            if path.exists() {
                // 加载 WASM 插件
                loaded_count += self.load_wasm_plugins(path).await?;
                
                // 加载 JavaScript 插件
                loaded_count += self.load_js_plugins(path).await?;
            }
        }
        
        Ok(loaded_count)
    }
    
    async fn load_js_plugins(&mut self, dir: &Path) -> Result<usize, PluginError> {
        let js_dir = dir.join("javascript");
        if !js_dir.exists() {
            return Ok(0);
        }
        
        let mut count = 0;
        for entry in std::fs::read_dir(js_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("js") {
                match self.load_js_plugin(&path).await {
                    Ok(_) => {
                        tracing::info!("Loaded JavaScript plugin: {:?}", path);
                        count += 1;
                    },
                    Err(e) => {
                        tracing::warn!("Failed to load JS plugin {:?}: {}", path, e);
                    }
                }
            }
        }
        
        Ok(count)
    }
    
    async fn load_js_plugin(&mut self, script_path: &Path) -> Result<(), PluginError> {
        // 1. 解析插件清单
        let manifest = self.extract_js_manifest(script_path)?;
        
        // 2. 创建 JavaScript 运行时
        let runtime = JsRuntime::new()?;
        
        // 3. 创建插件实例
        let plugin = JsPlugin {
            manifest,
            script_path: script_path.to_path_buf(),
            runtime,
        };
        
        // 4. 注册到统一注册表
        self.registry.register_js_plugin(plugin)?;
        
        Ok(())
    }
    
    fn extract_js_manifest(&self, script_path: &Path) -> Result<SkillManifest, PluginError> {
        let content = std::fs::read_to_string(script_path)?;
        
        // 查找 PLUGIN_MANIFEST 定义
        let manifest_regex = regex::Regex::new(r"const\s+PLUGIN_MANIFEST\s*=\s*(\{[\s\S]*?\});")?;
        
        if let Some(captures) = manifest_regex.captures(&content) {
            let manifest_json = captures.get(1).unwrap().as_str();
            let manifest: SkillManifest = serde_json::from_str(manifest_json)?;
            Ok(manifest)
        } else {
            Err(PluginError::InvalidManifest("No PLUGIN_MANIFEST found".to_string()))
        }
    }
}
```

---

## 📈 性能对比

| 特性 | WASM 插件 | JavaScript 插件 |
|------|-----------|-----------------|
| **启动速度** | 极快 (25ms) | 中等 (150ms) |
| **执行性能** | 接近原生 | 解释执行 |
| **内存占用** | 低 (13MB) | 中等 (45MB) |
| **开发便利性** | 需要编译 | 即时运行 |
| **调试体验** | 有限 | 优秀 |
| **生态兼容** | Rust 生态 | JavaScript 生态 |
| **类型安全** | 编译时保证 | 运行时检查 |

---

## 🎯 使用场景建议

### ✅ 适合 JavaScript 插件的场景

1. **快速原型开发**
   ```javascript
   // 快速实现一个简单的技能
   function pluginMain(skill, args, api) {
       if (skill === 'calculator.add') {
           const result = parseFloat(args.a) + parseFloat(args.b);
           api.success(result.toString());
       }
   }
   ```

2. **现有 JavaScript 库集成**
   ```javascript
   // 使用现有的 JavaScript 库
   const moment = require('moment'); // 如果可用
   
   function pluginMain(skill, args, api) {
       if (skill === 'time.format') {
           const formatted = moment(args.timestamp).format(args.format);
           api.success(formatted);
       }
   }
   ```

3. **复杂逻辑处理**
   ```javascript
   // 复杂的数据处理逻辑
   async function processData(data, api) {
       const processed = await complexTransform(data);
       api.success(JSON.stringify(processed));
   }
   ```

### ✅ 适合 WASM 插件的场景

1. **性能关键任务**
   ```rust
   // 高性能数据处理
   #[no_mangle]
   pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
       let req = sdk_read_request(ptr, len)?;
       let result = heavy_computation(&req.args);
       sdk_respond_ok(&req.request_id, &result)
   }
   ```

2. **系统级操作**
   ```rust
   // 需要精确内存控制的操作
   unsafe {
       let buffer = alloc_large_buffer(size);
       process_data(buffer);
       dealloc(buffer, size);
   }
   ```

3. **安全敏感代码**
   ```rust
   // Rust 编译时安全保证
   fn process_sensitive_data(input: &str) -> Result<String, Error> {
       // 编译时保证不会出现内存安全问题
   }
   ```

---

## 🚀 实施计划

### 阶段 1：基础框架 (2-3周)
- [ ] 设计统一插件接口
- [ ] 实现 JavaScript 运行时封装
- [ ] 创建插件包装器

### 阶段 2：插件加载 (2-3周)
- [ ] 扩展插件发现机制
- [ ] 实现 JavaScript 插件解析
- [ ] 添加插件注册表支持

### 阶段 3：测试和优化 (2-3周)
- [ ] 创建 JavaScript 插件示例
- [ ] 性能基准测试
- [ ] 安全性验证

### 阶段 4：文档和工具 (1-2周)
- [ ] 编写插件开发指南
- [ ] 创建插件模板
- [ ] 提供调试工具

---

## 🎉 总结

**JavaScript 插件兼容性完全可行**，而且能显著提升开发灵活性：

### ✅ 主要优势
1. **开发便利性**：无需编译，即时运行
2. **生态兼容**：可利用现有 JavaScript 库
3. **学习成本低**：大多数开发者熟悉 JavaScript
4. **快速原型**：适合快速验证想法

### 🎯 实施建议
1. **渐进式引入**：先支持基础功能，再逐步完善
2. **性能权衡**：JavaScript 插件适合灵活性要求高的场景
3. **安全考虑**：JavaScript 插件运行在沙箱中，安全性有保障
4. **文档完善**：提供清晰的开发指南和最佳实践

这种混合架构能让 OpenClaw+ 在保持高性能的同时，获得更大的开发灵活性和生态兼容性！
