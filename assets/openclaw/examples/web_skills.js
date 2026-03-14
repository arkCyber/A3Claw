/**
 * OpenClaw Web Skills — WasmEdge QuickJS Example
 *
 * Skills demonstrated:
 *   web.fetch, web_fetch (enhanced), web_search,
 *   search.web, search.query,
 *   web.navigate, web.click, web.fill, web.screenshot (stubs)
 */

import { SkillClient, writeLocalFile, logResult } from '../sdk/skills.js';

async function main() {
  print('[web_skills] === OpenClaw Web Skills Demo ===');

  var skills = new SkillClient({ gatewayUrl: 'http://127.0.0.1:7878' });

  // ── 1. web.fetch — GET request ────────────────────────────────────────────
  print('\n[1] web.fetch: GET https://httpbin.org/get');
  try {
    var r = await skills.webFetch('https://httpbin.org/get');
    logResult('web.fetch (GET)', r);
    print('    Status prefix: ' + r.slice(0, 10));
  } catch (e) { print('[web.fetch] ERROR: ' + e.message); }

  // ── 2. web.fetch — POST request with body ────────────────────────────────
  print('\n[2] web.fetch: POST https://httpbin.org/post');
  try {
    var r = await skills.webFetch(
      'https://httpbin.org/post',
      'POST',
      { 'Content-Type': 'application/json' },
      '{"agent":"openclaw"}'
    );
    logResult('web.fetch (POST)', r);
  } catch (e) { print('[web.fetch POST] ERROR: ' + e.message); }

  // ── 3. web_fetch (enhanced) — text extraction ────────────────────────────
  print('\n[3] web_fetch (enhanced): text mode https://example.com');
  try {
    var r = await skills.webFetchEnhanced('https://example.com', {
      extractMode: 'text',
      maxChars:    2000,
    });
    logResult('web_fetch (text)', r);
    print('    Has text: ' + (r.length > 10 ? 'OK (' + r.length + ' chars)' : 'EMPTY'));
  } catch (e) { print('[web_fetch enhanced] ERROR: ' + e.message); }

  // ── 4. web_fetch (enhanced) — markdown extraction ────────────────────────
  print('\n[4] web_fetch (enhanced): markdown mode https://example.com');
  try {
    var r = await skills.webFetchEnhanced('https://example.com', {
      extractMode: 'markdown',
      maxChars:    1000,
    });
    logResult('web_fetch (markdown)', r);
  } catch (e) { print('[web_fetch markdown] ERROR: ' + e.message); }

  // ── 5. web_search — Brave/DuckDuckGo search ──────────────────────────────
  print('\n[5] web_search: "OpenClaw AI agent"');
  try {
    var r = await skills.webSearch('OpenClaw AI agent', 3);
    logResult('web_search', r);
  } catch (e) { print('[web_search] ERROR: ' + e.message); }

  // ── 6. search.web — DuckDuckGo HTML scrape ───────────────────────────────
  print('\n[6] search.web: "WasmEdge QuickJS"');
  try {
    var r = await skills.searchWeb('WasmEdge QuickJS');
    logResult('search.web', r);
  } catch (e) { print('[search.web] ERROR: ' + e.message); }

  // ── 7. search.query — semantic alias ─────────────────────────────────────
  print('\n[7] search.query: "Rust async programming"');
  try {
    var r = await skills.searchQuery('Rust async programming');
    logResult('search.query', r);
  } catch (e) { print('[search.query] ERROR: ' + e.message); }

  // ── 8. web.navigate — browser stub ───────────────────────────────────────
  print('\n[8] web.navigate: https://openclaw.ai');
  try {
    var r = await skills.webNavigate('https://openclaw.ai');
    logResult('web.navigate', r);
  } catch (e) { print('[web.navigate] ERROR: ' + e.message); }

  // ── 9. web.click — browser stub ──────────────────────────────────────────
  print('\n[9] web.click: selector="#login-btn"');
  try {
    var r = await skills.webClick('#login-btn');
    logResult('web.click', r);
  } catch (e) { print('[web.click] ERROR: ' + e.message); }

  // ── 10. web.fill — browser form fill stub ────────────────────────────────
  print('\n[10] web.fill: selector="#email", value="user@example.com"');
  try {
    var r = await skills.webFill('#email', 'user@example.com');
    logResult('web.fill', r);
  } catch (e) { print('[web.fill] ERROR: ' + e.message); }

  // ── 11. web.screenshot — browser screenshot stub ─────────────────────────
  print('\n[11] web.screenshot: https://example.com (stub)');
  try {
    var r = await skills.webScreenshot('https://example.com', { width: 1280, height: 800 });
    logResult('web.screenshot', r);
  } catch (e) { print('[web.screenshot] ERROR: ' + e.message); }

  // ── Summary ───────────────────────────────────────────────────────────────
  print('\n[web_skills] === All Web Skills Demo Complete ===');

  try {
    await writeLocalFile('/workspace/web_skills_result.txt',
      'OpenClaw Web Skills Demo\n' +
      'Completed: ' + new Date().toISOString() + '\n' +
      'Skills tested: web.fetch (GET/POST), web_fetch (text/markdown),\n' +
      '               web_search, search.web, search.query,\n' +
      '               web.navigate, web.click, web.fill, web.screenshot\n'
    );
    print('[web_skills] Result written to /workspace/web_skills_result.txt');
  } catch (e) { print('[writeLocalFile] ERROR: ' + e.message); }
}

main().catch(function(e) { print('[FATAL] ' + e.message); });
