# OpenClaw Skills Guide — WasmEdge Edition

All 78 built-in skills, their implementation status, and how to run them
from a WasmEdge QuickJS sandbox.

---

## Architecture

```
WasmEdge QuickJS sandbox
  └── your-script.js
        └── import { SkillClient } from './sdk/skills.js'
              └── HTTP POST /skill/execute  →  Gateway (port 7878)
                    ├── fs.*, exec         (inline std::fs + std::process)
                    ├── security.*, loop_detection.*, gateway.*
                    │   canvas.present/snapshot, nodes.notify/run
                    │   message.send, agents.list
                    └── stubs: email.*, calendar.*, canvas.*, nodes.*,
                               knowledge.*, cron.*, sessions.*, process.*,
                               web.fetch, web_fetch, web_search, image
                               (these require the AgentExecutor process)
```

---

## Skill Classification

### Tier 1 — Fully implemented in Gateway `/skill/execute`

These skills execute inline in the Gateway without needing the executor.

| Skill | Args | Notes |
|-------|------|-------|
| `fs.readFile` | `path` | Returns up to 3000 chars |
| `fs.readDir` | `path` | Newline-separated names |
| `fs.stat` | `path` | `size, is_dir, is_file` |
| `fs.exists` | `path` | Returns `"true"` or `"false"` |
| `fs.writeFile` | `path`, `content` | Overwrites |
| `fs.mkdir` | `path` | Creates parents (`create_dir_all`) |
| `fs.deleteFile` | `path` | Removes single file |
| `fs.move` | `src`, `dest` | Renames |
| `fs.copy` | `src`, `dest` | Copies file |
| `exec` | `command`, `cwd?`, `env?`, `timeout_secs?` | Sync shell via `sh -c` |
| `agent.listSkills` | — | Returns JSON array of grantable skills |
| `security.getStatus` | — | Breaker + event counters |
| `security.listEvents` | `limit?` | Recent audit ring buffer |
| `loop_detection.status` | — | Event stats |
| `loop_detection.reset` | — | Reset counters |
| `gateway.config.get` | — | Current config JSON |
| `gateway.restart` | `delayMs?` | Schedules restart |
| `gateway.update.run` | — | Triggers update check |
| `canvas.present` | `content`, `node?` | Records to audit log |
| `canvas.snapshot` | `node?` | Stub (companion app required) |
| `nodes.status` | — | Active session count |
| `nodes.notify` | `title`, `message`, `node?` | Queues notification |
| `nodes.run` | `node`, `command` | Stub (companion app required) |
| `message.send` | `channel`, `text`, `platform?` | Queues message |
| `agents.list` | — | Active session count |

---

### Tier 2 — Implemented in AgentExecutor (not in Gateway)

These require the full executor process (`openclaw-agent-executor`).
Call them via the standard ReAct loop, not through `/skill/execute` directly.

| Skill | Category | Notes |
|-------|----------|-------|
| `web.fetch` | Web | HTTP GET/POST via reqwest |
| `web_fetch` | Web | Enhanced fetch with HTML→text extraction |
| `web_search` | Web | Brave API or DuckDuckGo fallback |
| `web.navigate` | Browser | Stub — requires headless browser |
| `web.click` | Browser | Stub |
| `web.fill` | Browser | Stub |
| `web.screenshot` | Browser | Stub |
| `search.web` | Search | DuckDuckGo HTML scrape |
| `search.query` | Search | Alias for search.web |
| `knowledge.query` | Knowledge | Falls back to agent memory |
| `knowledge.retrieve` | Knowledge | Alias for knowledge.query |
| `agent.getContext` | Agent | TaskContext summary |
| `agent.getMemory` | Agent | Key lookup in TaskContext.memory |
| `agent.setMemory` | Agent | Store in TaskContext.memory |
| `agent.clearMemory` | Agent | Clear all TaskContext.memory |
| `agent.delegate` | Agent | Sub-task via Gateway /agent/delegate |
| `apply_patch` | FS | Unified diff or search-replace patch |
| `process.list` | Process | List background sessions |
| `process.poll` | Process | Poll session for new output |
| `process.log` | Process | Read stdout lines |
| `process.kill` | Process | Kill background session |
| `process.clear` | Process | Remove completed sessions |
| `cron.status` | Cron | Job count |
| `cron.list` | Cron | All jobs JSON |
| `cron.add` | Cron | Add job (schedule + goal) |
| `cron.remove` | Cron | Remove by jobId |
| `cron.run` | Cron | Manual trigger |
| `sessions.list` | Session | Via Gateway /sessions |
| `sessions.history` | Session | Via Gateway /sessions/:id/history |
| `sessions.send` | Session | Via Gateway /sessions/:id/send |
| `sessions.spawn` | Session | Via Gateway /sessions/spawn |
| `session.status` | Session | Via Gateway /sessions/:id/status |
| `image` | Vision | Requires vision model |

---

### Tier 3 — Stubs (require external configuration)

These return a descriptive hint until a `SkillHandler` is registered.

| Skill | Handler required | Config needed |
|-------|------------------|---------------|
| `email.list` | `EmailSkillHandler` | IMAP server + credentials |
| `email.read` | `EmailSkillHandler` | IMAP |
| `email.send` | `EmailSkillHandler` | SMTP server + credentials |
| `email.reply` | `EmailSkillHandler` | SMTP |
| `email.delete` | `EmailSkillHandler` | IMAP |
| `calendar.list` | `CalendarSkillHandler` | CalDAV server |
| `calendar.create` | `CalendarSkillHandler` | CalDAV |
| `calendar.get` | `CalendarSkillHandler` | CalDAV |
| `calendar.update` | `CalendarSkillHandler` | CalDAV |
| `calendar.delete` | `CalendarSkillHandler` | CalDAV |
| `canvas.create` | `CanvasSkillHandler` | Canvas backend |
| `canvas.get` | `CanvasSkillHandler` | |
| `canvas.update` | `CanvasSkillHandler` | |
| `canvas.list` | `CanvasSkillHandler` | |
| `canvas.delete` | `CanvasSkillHandler` | |
| `canvas.export` | `CanvasSkillHandler` | |
| `nodes.list` | companion app | macOS OpenClaw+ app |
| `nodes.add` | companion app | |
| `nodes.remove` | companion app | |
| `nodes.connect` | companion app | |
| `nodes.disconnect` | companion app | |
| `message.reply` | `MessageSkillHandler` | Discord/Slack/Teams token |
| `message.react` | `MessageSkillHandler` | |
| `message.delete` | `MessageSkillHandler` | |
| `message.read` | `MessageSkillHandler` | |
| `message.search` | `MessageSkillHandler` | |
| `gateway.config.schema` | — | Built-in (returns JSON schema) |
| `gateway.config.patch` | — | Built-in |

---

## Running Examples in WasmEdge

### Prerequisites

1. Start the OpenClaw Gateway:
   ```bash
   cargo run -p openclaw-plugin-gateway
   # Default: http://127.0.0.1:7878
   ```

2. Build the WasmEdge QuickJS runtime (wasmedge_quickjs.wasm):
   ```bash
   # Follow WasmEdge QuickJS build instructions
   # Or download from https://github.com/second-state/wasmedge-quickjs/releases
   ```

3. Run a skill example:
   ```bash
   wasmedge --dir /workspace:/workspace \
            wasmedge_quickjs.wasm \
            assets/openclaw/examples/fs_skills.js
   ```

### Example Scripts

| Script | Skills covered |
|--------|----------------|
| `examples/fs_skills.js` | `fs.mkdir`, `fs.writeFile`, `fs.readFile`, `fs.stat`, `fs.exists`, `fs.copy`, `fs.move`, `fs.readDir`, `apply_patch`, `fs.deleteFile` |
| `examples/exec_skills.js` | `exec` (sync/bg/cwd/env), `process.list/poll/log/kill/clear` |
| `examples/web_skills.js` | `web.fetch` (GET/POST), `web_fetch`, `web_search`, `search.web`, `search.query`, `web.navigate`, `web.click`, `web.fill`, `web.screenshot` |
| `examples/agent_skills.js` | `agent.listSkills/setMemory/getMemory/clearMemory/getContext/delegate`, `knowledge.query/retrieve`, `security.*`, `loop_detection.*`, `image` |
| `examples/cron_skills.js` | `cron.status`, `cron.list`, `cron.add` (×3), `cron.run` (×2), `cron.remove` |
| `examples/sessions_skills.js` | `agents.list`, `sessions.list/spawn/send/history`, `session.status`, `gateway.*` |
| `examples/messaging_skills.js` | `message.*` (6), `canvas.*` (8), `nodes.*` (8), `email.*` (5, stubs), `calendar.*` (5, stubs) |

### Using the SDK in your own script

```js
import { SkillClient, writeLocalFile, logResult } from './sdk/skills.js';

async function main() {
  const skills = new SkillClient({ gatewayUrl: 'http://127.0.0.1:7878' });

  // File system
  await skills.fsMkdir('/workspace/output');
  await skills.fsWriteFile('/workspace/output/report.txt', 'Hello!');
  const content = await skills.fsReadFile('/workspace/output/report.txt');
  print('[fs.readFile] ' + content);

  // Shell exec
  const out = await skills.exec('ls -la /workspace');
  print('[exec] ' + out);

  // Security status
  const status = await skills.securityGetStatus();
  print('[security] ' + status);

  // Write final result directly (no Gateway needed)
  await writeLocalFile('/workspace/done.txt', 'Completed: ' + new Date().toISOString());
}

main().catch(e => print('[FATAL] ' + e.message));
```

---

## WasmEdge QuickJS Available APIs

| API | Import | Status |
|-----|--------|--------|
| TLS networking | `import * as net from 'wasi_net'` | ✅ `WasiTlsConn.connect(host, 443)` |
| HTTP utils | `import * as wHttp from 'wasi_http'` | ✅ `WasiRequest`, `WasiResponse` |
| File write | `import * as std from 'std'; std.open(path,'w').puts(txt)` | ✅ |
| `print()` / `console.log()` | built-in | ✅ stdout |
| `require()` | — | ❌ not available |
| `fetch()` | — | ❌ not available |
| `node:fs` | `import * as fs from '_node:fs'` | ❌ `openSync` returns undefined |
| `TextDecoder` / `URL` | — | ❌ not available |

---

## Test Coverage Summary

| Crate | Tests | Skill coverage |
|-------|-------|----------------|
| `agent-executor` | 321 | dispatch-level tests for all 78 skills |
| `openclaw-plugin-gateway` | 104 | `/skill/execute` endpoint (16 new tests) + all existing routes |
| `openclaw-security` | 201 | Policy engine, audit, sandbox |
| `sandbox` | 45 | Host functions, wasi builder |

**All 78 skills have at least one test in `dispatch.rs` or `router.rs`.**
