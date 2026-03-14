/**
 * OpenClaw Skills SDK for WasmEdge QuickJS
 *
 * Provides a unified interface for calling all 78 OpenClaw built-in skills
 * from inside a WasmEdge QuickJS sandbox.  Communication goes through the
 * OpenClaw Gateway HTTP API (default: http://127.0.0.1:7878).
 *
 * Usage:
 *   import { SkillClient } from './sdk/skills.js';
 *   const skills = new SkillClient({ gatewayUrl: 'http://127.0.0.1:7878' });
 *   const result = await skills.call('fs.readFile', { path: '/workspace/data.txt' });
 *
 * WasmEdge QuickJS network API:
 *   import * as net from 'wasi_net';
 *   conn = await net.WasiTlsConn.connect(host, 443)   // HTTPS
 *   // For HTTP use the plain TCP variant if available, otherwise use Gateway proxy
 *
 * NOTE: All Gateway calls use plain HTTP (port 7878 by default) so we use
 *       the simple TCP approach via wasi_net for the loopback connection.
 */

// ── Low-level HTTP/1.1 over wasi_net ─────────────────────────────────────────

/**
 * Minimal HTTP/1.1 POST over TCP (plain, for loopback Gateway calls).
 * Returns { status: number, body: string }.
 */
async function httpPost(host, port, path, jsonBody) {
  var net = await import('wasi_net');
  var body = typeof jsonBody === 'string' ? jsonBody : JSON.stringify(jsonBody);
  var req = (
    'POST ' + path + ' HTTP/1.1\r\n' +
    'Host: ' + host + ':' + port + '\r\n' +
    'Content-Type: application/json\r\n' +
    'Content-Length: ' + body.length + '\r\n' +
    'Connection: close\r\n' +
    '\r\n' +
    body
  );

  var enc = new Uint8Array(req.length);
  for (var i = 0; i < req.length; i++) enc[i] = req.charCodeAt(i) & 0xff;

  var conn = await net.WasiTcpConn.connect(host, port);
  await conn.write(enc.buffer);

  var chunks = [];
  var total = 0;
  while (total < 1024 * 1024) {
    var chunk = await conn.read();
    if (!chunk || chunk.byteLength === 0) break;
    chunks.push(new Uint8Array(chunk));
    total += chunk.byteLength;
  }

  var all = new Uint8Array(total);
  var off = 0;
  for (var i = 0; i < chunks.length; i++) {
    all.set(chunks[i], off);
    off += chunks[i].length;
  }

  var text = '';
  for (var i = 0; i < all.length; i++) text += String.fromCharCode(all[i]);

  var sep = text.indexOf('\r\n\r\n');
  if (sep === -1) sep = text.indexOf('\n\n');
  var header = sep !== -1 ? text.slice(0, sep) : text;
  var responseBody = sep !== -1 ? text.slice(sep + 4) : '';
  var firstLine = header.split('\n')[0].trim();
  var status = parseInt(firstLine.split(' ')[1] || '0', 10);

  return { status: status, body: responseBody };
}

/**
 * HTTP GET over TCP (plain, for loopback Gateway calls).
 */
async function httpGet(host, port, path) {
  var net = await import('wasi_net');
  var req = (
    'GET ' + path + ' HTTP/1.1\r\n' +
    'Host: ' + host + ':' + port + '\r\n' +
    'Connection: close\r\n' +
    '\r\n'
  );

  var enc = new Uint8Array(req.length);
  for (var i = 0; i < req.length; i++) enc[i] = req.charCodeAt(i) & 0xff;

  var conn = await net.WasiTcpConn.connect(host, port);
  await conn.write(enc.buffer);

  var chunks = [];
  var total = 0;
  while (total < 1024 * 1024) {
    var chunk = await conn.read();
    if (!chunk || chunk.byteLength === 0) break;
    chunks.push(new Uint8Array(chunk));
    total += chunk.byteLength;
  }

  var all = new Uint8Array(total);
  var off = 0;
  for (var i = 0; i < chunks.length; i++) {
    all.set(chunks[i], off);
    off += chunks[i].length;
  }

  var text = '';
  for (var i = 0; i < all.length; i++) text += String.fromCharCode(all[i]);

  var sep = text.indexOf('\r\n\r\n');
  if (sep === -1) sep = text.indexOf('\n\n');
  var header = sep !== -1 ? text.slice(0, sep) : text;
  var responseBody = sep !== -1 ? text.slice(sep + 4) : '';
  var firstLine = header.split('\n')[0].trim();
  var status = parseInt(firstLine.split(' ')[1] || '0', 10);

  return { status: status, body: responseBody };
}

// ── Skill Client ──────────────────────────────────────────────────────────────

/**
 * Main skill client.  Each method corresponds to one OpenClaw built-in skill.
 *
 * @param {object} opts
 * @param {string} opts.gatewayUrl  - Gateway base URL, e.g. 'http://127.0.0.1:7878'
 * @param {string} opts.sessionId   - Agent session ID (set by runner, can override)
 */
export function SkillClient(opts) {
  opts = opts || {};
  this._gatewayUrl = opts.gatewayUrl || 'http://127.0.0.1:7878';
  this._sessionId  = opts.sessionId  || 'wasm-session-' + Date.now();
  var parsed = parseUrl(this._gatewayUrl);
  this._host = parsed.host;
  this._port = parsed.port;
}

function parseUrl(url) {
  // Very minimal URL parser for http://host:port
  var m = url.match(/^https?:\/\/([^:/]+)(?::(\d+))?/);
  if (!m) return { host: '127.0.0.1', port: 7878 };
  return { host: m[1], port: parseInt(m[2] || '7878', 10) };
}

/**
 * Execute any skill by name with args object.
 * Returns the skill output string (same as SkillDispatcher::execute_skill).
 */
SkillClient.prototype.call = async function(skillName, args) {
  var payload = {
    skill: skillName,
    args:  args || {},
    sessionId: this._sessionId,
  };
  var resp = await httpPost(this._host, this._port, '/skill/execute', payload);
  if (resp.status === 200) {
    try {
      var parsed = JSON.parse(resp.body);
      return parsed.output !== undefined ? parsed.output : resp.body;
    } catch (e) {
      return resp.body;
    }
  }
  throw new Error('Skill ' + skillName + ' failed: HTTP ' + resp.status + ' ' + resp.body.slice(0, 200));
};

// ── File System Skills ────────────────────────────────────────────────────────

/** Read file contents.  Returns string. */
SkillClient.prototype.fsReadFile = async function(path, encoding) {
  return this.call('fs.readFile', { path: path, encoding: encoding || 'utf8' });
};

/** List directory entries.  Returns newline-separated names. */
SkillClient.prototype.fsReadDir = async function(path) {
  return this.call('fs.readDir', { path: path });
};

/** Get file/directory metadata.  Returns "size: N bytes, is_dir: X, is_file: Y". */
SkillClient.prototype.fsStat = async function(path) {
  return this.call('fs.stat', { path: path });
};

/** Check existence.  Returns "true" or "false". */
SkillClient.prototype.fsExists = async function(path) {
  return this.call('fs.exists', { path: path });
};

/** Write (overwrite) file with content string. */
SkillClient.prototype.fsWriteFile = async function(path, content, encoding) {
  return this.call('fs.writeFile', { path: path, content: content, encoding: encoding || 'utf8' });
};

/** Create directory (including parents). */
SkillClient.prototype.fsMkdir = async function(path, recursive) {
  return this.call('fs.mkdir', { path: path, recursive: recursive !== false });
};

/** Delete a file. */
SkillClient.prototype.fsDeleteFile = async function(path) {
  return this.call('fs.deleteFile', { path: path });
};

/** Move/rename file or directory. */
SkillClient.prototype.fsMove = async function(src, dest) {
  return this.call('fs.move', { src: src, dest: dest });
};

/** Copy file. */
SkillClient.prototype.fsCopy = async function(src, dest) {
  return this.call('fs.copy', { src: src, dest: dest });
};

/** Apply unified diff or search-replace patch. */
SkillClient.prototype.applyPatch = async function(patchText, workspaceRoot) {
  return this.call('apply_patch', { patch: patchText, workspace_root: workspaceRoot });
};

// ── Shell / Process Skills ────────────────────────────────────────────────────

/** Execute a shell command.  Returns stdout+stderr. */
SkillClient.prototype.exec = async function(command, opts) {
  opts = opts || {};
  return this.call('exec', {
    command:      command,
    cwd:          opts.cwd,
    timeout_secs: opts.timeoutSecs || 30,
    background:   opts.background  || false,
    env:          opts.env,
  });
};

/** Execute in background, returns sessionId string. */
SkillClient.prototype.execBackground = async function(command, opts) {
  opts = opts || {};
  return this.call('exec', {
    command:    command,
    cwd:        opts.cwd,
    background: true,
    env:        opts.env,
  });
};

/** List all background process sessions. */
SkillClient.prototype.processList = async function() {
  return this.call('process.list', {});
};

/** Poll a background process session for new output. */
SkillClient.prototype.processPoll = async function(sessionId) {
  return this.call('process.poll', { sessionId: sessionId });
};

/** Read stdout lines from a background process. */
SkillClient.prototype.processLog = async function(sessionId, offset, limit) {
  return this.call('process.log', {
    sessionId: sessionId,
    offset:    offset || 0,
    limit:     limit  || 100,
  });
};

/** Kill a background process session. */
SkillClient.prototype.processKill = async function(sessionId) {
  return this.call('process.kill', { sessionId: sessionId });
};

/** Remove all completed background process sessions. */
SkillClient.prototype.processClear = async function() {
  return this.call('process.clear', {});
};

// ── Web Skills ────────────────────────────────────────────────────────────────

/** Simple URL fetch.  Returns "HTTP NNN\n<body>". */
SkillClient.prototype.webFetch = async function(url, method, headers, body) {
  return this.call('web.fetch', {
    url:     url,
    method:  method  || 'GET',
    headers: headers || {},
    body:    body,
  });
};

/** Enhanced fetch with HTML-to-text extraction. */
SkillClient.prototype.webFetchEnhanced = async function(url, opts) {
  opts = opts || {};
  return this.call('web_fetch', {
    url:          url,
    extract_mode: opts.extractMode || 'text',
    max_chars:    opts.maxChars    || 8000,
    method:       opts.method      || 'GET',
    headers:      opts.headers     || {},
    body:         opts.body,
  });
};

/** Web search via Brave API (requires BRAVE_API_KEY) or DuckDuckGo fallback. */
SkillClient.prototype.webSearch = async function(query, count) {
  return this.call('web_search', { query: query, count: count || 5 });
};

/** Browser navigate stub. */
SkillClient.prototype.webNavigate = async function(url) {
  return this.call('web.navigate', { url: url });
};

/** Browser click stub. */
SkillClient.prototype.webClick = async function(selector) {
  return this.call('web.click', { selector: selector });
};

/** Browser form fill stub. */
SkillClient.prototype.webFill = async function(selector, value) {
  return this.call('web.fill', { selector: selector, value: value });
};

/** Screenshot stub (requires headless browser backend). */
SkillClient.prototype.webScreenshot = async function(url, opts) {
  opts = opts || {};
  return this.call('web.screenshot', {
    url:    url,
    width:  opts.width  || 1280,
    height: opts.height || 800,
  });
};

// ── Search Skills ─────────────────────────────────────────────────────────────

/** Search via DuckDuckGo HTML scrape. */
SkillClient.prototype.searchWeb = async function(query) {
  return this.call('search.web', { query: query });
};

/** Semantic search query (alias for search.web). */
SkillClient.prototype.searchQuery = async function(query) {
  return this.call('search.query', { query: query });
};

// ── Knowledge / RAG Skills ────────────────────────────────────────────────────

/** Query the knowledge base (falls back to agent memory). */
SkillClient.prototype.knowledgeQuery = async function(question) {
  return this.call('knowledge.query', { question: question });
};

/** Retrieve from knowledge base (alias). */
SkillClient.prototype.knowledgeRetrieve = async function(query) {
  return this.call('knowledge.retrieve', { query: query });
};

// ── Agent Memory / Introspection Skills ───────────────────────────────────────

/** Get a memory key (or list all keys if key is empty). */
SkillClient.prototype.agentGetMemory = async function(key) {
  return this.call('agent.getMemory', { key: key || '' });
};

/** Set a memory key-value pair. */
SkillClient.prototype.agentSetMemory = async function(key, value) {
  return this.call('agent.setMemory', { key: key, value: value });
};

/** Clear all agent memory. */
SkillClient.prototype.agentClearMemory = async function() {
  return this.call('agent.clearMemory', {});
};

/** List all grantable skills. */
SkillClient.prototype.agentListSkills = async function() {
  return this.call('agent.listSkills', {});
};

/** Get a summary of the current task context. */
SkillClient.prototype.agentGetContext = async function() {
  return this.call('agent.getContext', {});
};

/** Delegate a sub-task to another agent. */
SkillClient.prototype.agentDelegate = async function(agentId, goal, timeoutSecs) {
  return this.call('agent.delegate', {
    agent_id:     agentId,
    goal:         goal,
    timeout_secs: timeoutSecs || 60,
  });
};

// ── Cron Scheduler Skills ─────────────────────────────────────────────────────

/** Return cron scheduler status. */
SkillClient.prototype.cronStatus = async function() {
  return this.call('cron.status', {});
};

/** List all cron jobs. */
SkillClient.prototype.cronList = async function() {
  return this.call('cron.list', {});
};

/**
 * Add a new cron job.
 * @param {string} name     - Job name
 * @param {string} schedule - Cron expression (5-field or @hourly/@daily etc.)
 * @param {string} goal     - Goal to run on schedule
 * @param {boolean} enabled - Whether job is active (default true)
 */
SkillClient.prototype.cronAdd = async function(name, schedule, goal, enabled) {
  return this.call('cron.add', {
    name:     name,
    schedule: schedule,
    goal:     goal,
    enabled:  enabled !== false,
  });
};

/** Remove a cron job by ID. */
SkillClient.prototype.cronRemove = async function(jobId) {
  return this.call('cron.remove', { jobId: jobId });
};

/** Manually trigger a cron job. */
SkillClient.prototype.cronRun = async function(jobId) {
  return this.call('cron.run', { jobId: jobId });
};

// ── Session Management Skills ─────────────────────────────────────────────────

/** List active/recent agent sessions. */
SkillClient.prototype.sessionsList = async function(limit) {
  return this.call('sessions.list', { limit: limit || 20 });
};

/** Fetch message history for a specific session. */
SkillClient.prototype.sessionsHistory = async function(sessionId, limit) {
  return this.call('sessions.history', { sessionId: sessionId, limit: limit || 50 });
};

/** Send a message into an existing agent session. */
SkillClient.prototype.sessionsSend = async function(sessionId, message) {
  return this.call('sessions.send', { sessionId: sessionId, message: message });
};

/** Spawn a new agent session. */
SkillClient.prototype.sessionsSpawn = async function(agentId, goal) {
  return this.call('sessions.spawn', { agentId: agentId, goal: goal || '' });
};

/** Get status of a specific session. */
SkillClient.prototype.sessionStatus = async function(sessionId) {
  return this.call('session.status', { sessionId: sessionId });
};

/** List all registered agents. */
SkillClient.prototype.agentsList = async function() {
  return this.call('agents.list', {});
};

// ── Email Skills (requires EmailSkillHandler) ─────────────────────────────────

/** List emails from inbox. */
SkillClient.prototype.emailList = async function(opts) {
  opts = opts || {};
  return this.call('email.list', { folder: opts.folder || 'INBOX', limit: opts.limit || 20 });
};

/** Read a specific email. */
SkillClient.prototype.emailRead = async function(emailId) {
  return this.call('email.read', { email_id: emailId });
};

/** Send an email. */
SkillClient.prototype.emailSend = async function(to, subject, body) {
  return this.call('email.send', { to: to, subject: subject, body: body });
};

/** Reply to an email. */
SkillClient.prototype.emailReply = async function(emailId, body) {
  return this.call('email.reply', { email_id: emailId, body: body });
};

/** Delete an email. */
SkillClient.prototype.emailDelete = async function(emailId) {
  return this.call('email.delete', { email_id: emailId });
};

// ── Calendar Skills (requires CalendarSkillHandler) ───────────────────────────

/** List calendar events. */
SkillClient.prototype.calendarList = async function(start, end) {
  return this.call('calendar.list', { start: start, end: end });
};

/** Create a calendar event. */
SkillClient.prototype.calendarCreate = async function(title, start, end, opts) {
  opts = opts || {};
  return this.call('calendar.create', {
    title:       title,
    start:       start,
    end:         end,
    description: opts.description || '',
    location:    opts.location    || '',
  });
};

/** Get a specific calendar event. */
SkillClient.prototype.calendarGet = async function(eventId) {
  return this.call('calendar.get', { event_id: eventId });
};

/** Update a calendar event. */
SkillClient.prototype.calendarUpdate = async function(eventId, updates) {
  return this.call('calendar.update', Object.assign({ event_id: eventId }, updates));
};

/** Delete a calendar event. */
SkillClient.prototype.calendarDelete = async function(eventId) {
  return this.call('calendar.delete', { event_id: eventId });
};

// ── Security Skills ───────────────────────────────────────────────────────────

/** Get sandbox security status. */
SkillClient.prototype.securityGetStatus = async function() {
  return this.call('security.getStatus', {});
};

/** List recent security audit events. */
SkillClient.prototype.securityListEvents = async function(limit) {
  return this.call('security.listEvents', { limit: limit || 20 });
};

// ── Loop Detection Skills ─────────────────────────────────────────────────────

/** Get loop detection status. */
SkillClient.prototype.loopDetectionStatus = async function() {
  return this.call('loop_detection.status', {});
};

/** Reset loop detection counters. */
SkillClient.prototype.loopDetectionReset = async function() {
  return this.call('loop_detection.reset', {});
};

// ── Canvas Skills (requires CanvasSkillHandler) ───────────────────────────────

/** Create a new canvas. */
SkillClient.prototype.canvasCreate = async function(title, type) {
  return this.call('canvas.create', { title: title, type: type || 'whiteboard' });
};

/** Get a canvas by ID. */
SkillClient.prototype.canvasGet = async function(canvasId) {
  return this.call('canvas.get', { canvas_id: canvasId });
};

/** Update a canvas. */
SkillClient.prototype.canvasUpdate = async function(canvasId, content) {
  return this.call('canvas.update', { canvas_id: canvasId, content: content });
};

/** List all canvases. */
SkillClient.prototype.canvasList = async function() {
  return this.call('canvas.list', {});
};

/** Delete a canvas. */
SkillClient.prototype.canvasDelete = async function(canvasId) {
  return this.call('canvas.delete', { canvas_id: canvasId });
};

/** Export a canvas. */
SkillClient.prototype.canvasExport = async function(canvasId, format) {
  return this.call('canvas.export', { canvas_id: canvasId, format: format || 'markdown' });
};

/** Present content to a canvas node. */
SkillClient.prototype.canvasPresent = async function(content, node) {
  return this.call('canvas.present', { content: content, node: node });
};

/** Snapshot a canvas node. */
SkillClient.prototype.canvasSnapshot = async function(node) {
  return this.call('canvas.snapshot', { node: node });
};

// ── Nodes Skills (macOS Companion App) ───────────────────────────────────────

/** List connected nodes. */
SkillClient.prototype.nodesList = async function() {
  return this.call('nodes.list', {});
};

/** Add a node. */
SkillClient.prototype.nodesAdd = async function(type, opts) {
  return this.call('nodes.add', Object.assign({ type: type }, opts || {}));
};

/** Remove a node. */
SkillClient.prototype.nodesRemove = async function(node) {
  return this.call('nodes.remove', { node: node });
};

/** Connect two nodes. */
SkillClient.prototype.nodesConnect = async function(from, to) {
  return this.call('nodes.connect', { from: from, to: to });
};

/** Disconnect two nodes. */
SkillClient.prototype.nodesDisconnect = async function(from, to) {
  return this.call('nodes.disconnect', { from: from, to: to });
};

/** Get node status. */
SkillClient.prototype.nodesStatus = async function() {
  return this.call('nodes.status', {});
};

/** Send a push notification to a node. */
SkillClient.prototype.nodesNotify = async function(title, message, node) {
  return this.call('nodes.notify', { title: title, message: message, node: node });
};

/** Run a command on a remote node. */
SkillClient.prototype.nodesRun = async function(node, command, opts) {
  opts = opts || {};
  return this.call('nodes.run', { node: node, command: command, cwd: opts.cwd });
};

// ── Message Skills (requires MessageSkillHandler) ─────────────────────────────

/** Send a message to a channel. */
SkillClient.prototype.messageSend = async function(channel, text, platform) {
  return this.call('message.send', {
    channel:  channel,
    text:     text,
    platform: platform || 'discord',
  });
};

/** Reply to a message. */
SkillClient.prototype.messageReply = async function(messageId, text, channel) {
  return this.call('message.reply', { message_id: messageId, text: text, channel: channel });
};

/** React to a message with an emoji. */
SkillClient.prototype.messageReact = async function(messageId, emoji, channel) {
  return this.call('message.react', { message_id: messageId, emoji: emoji, channel: channel });
};

/** Delete a message. */
SkillClient.prototype.messageDelete = async function(messageId, channel) {
  return this.call('message.delete', { message_id: messageId, channel: channel });
};

/** Read a specific message. */
SkillClient.prototype.messageRead = async function(messageId, channel) {
  return this.call('message.read', { message_id: messageId, channel: channel });
};

/** Search messages. */
SkillClient.prototype.messageSearch = async function(query, channel) {
  return this.call('message.search', { query: query, channel: channel });
};

// ── Image Analysis Skill ──────────────────────────────────────────────────────

/** Analyze an image (local path or URL) using vision model. */
SkillClient.prototype.imageAnalyze = async function(imagePath, prompt, opts) {
  opts = opts || {};
  return this.call('image', {
    image:        imagePath,
    prompt:       prompt || 'Describe this image.',
    model:        opts.model,
    max_bytes_mb: opts.maxBytesMb || 10,
  });
};

// ── Gateway Management Skills ─────────────────────────────────────────────────

/** Restart the Gateway. */
SkillClient.prototype.gatewayRestart = async function(delayMs) {
  return this.call('gateway.restart', { delayMs: delayMs || 0 });
};

/** Get current Gateway configuration. */
SkillClient.prototype.gatewayConfigGet = async function() {
  return this.call('gateway.config.get', {});
};

/** Get Gateway configuration JSON schema. */
SkillClient.prototype.gatewayConfigSchema = async function() {
  return this.call('gateway.config.schema', {});
};

/** Patch Gateway configuration. */
SkillClient.prototype.gatewayConfigPatch = async function(patch) {
  return this.call('gateway.config.patch', { patch: patch });
};

/** Run a Gateway update. */
SkillClient.prototype.gatewayUpdateRun = async function() {
  return this.call('gateway.update.run', {});
};

// ── Utility: write file (std module, no Gateway needed) ───────────────────────

/**
 * Write a file directly using QuickJS std module (bypasses Gateway).
 * Use this for writing output files from within the WASM sandbox.
 */
export async function writeLocalFile(path, content) {
  var std = await import('std');
  var f = std.open(path, 'w');
  if (!f) throw new Error('std.open failed for ' + path);
  f.puts(content);
  f.close();
}

/**
 * Read a file directly using QuickJS std module (bypasses Gateway).
 */
export function readLocalFile(path) {
  var std = require ? require('std') : null;
  // QuickJS std is synchronous
  if (typeof std === 'undefined' || !std) {
    throw new Error('std module not available');
  }
  var f = std.open(path, 'r');
  if (!f) throw new Error('std.open failed for ' + path);
  var content = f.readAsString();
  f.close();
  return content;
}

/** Print a labelled result for logging. */
export function logResult(skillName, result) {
  print('[' + skillName + '] ' + (typeof result === 'string' ? result.slice(0, 200) : JSON.stringify(result)));
}
