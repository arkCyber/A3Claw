/**
 * OpenClaw Exec / Process Skills — WasmEdge QuickJS Example
 *
 * Skills demonstrated:
 *   exec  (sync + background)
 *   process.list, process.poll, process.log, process.kill, process.clear
 */

import { SkillClient, writeLocalFile, logResult } from '../sdk/skills.js';

async function main() {
  print('[exec_skills] === OpenClaw Exec / Process Skills Demo ===');

  var skills = new SkillClient({ gatewayUrl: 'http://127.0.0.1:7878' });

  // ── 1. exec (sync) — simple echo ─────────────────────────────────────────
  print('\n[1] exec (sync): echo hello');
  try {
    var r = await skills.exec('echo "OpenClaw exec skill test"');
    logResult('exec (sync)', r);
    print('    Output has "OpenClaw": ' + (r.includes('OpenClaw') ? 'OK' : 'FAIL'));
  } catch (e) { print('[exec sync] ERROR: ' + e.message); }

  // ── 2. exec (sync) — multi-command pipeline ───────────────────────────────
  print('\n[2] exec (sync): pipeline');
  try {
    var r = await skills.exec('echo -e "apple\\nbanana\\ncherry" | sort | head -2');
    logResult('exec (pipeline)', r);
  } catch (e) { print('[exec pipeline] ERROR: ' + e.message); }

  // ── 3. exec (sync) — run in specific cwd ─────────────────────────────────
  print('\n[3] exec (sync): pwd in /workspace');
  try {
    var r = await skills.exec('pwd', { cwd: '/workspace' });
    logResult('exec (cwd)', r);
    print('    Has /workspace: ' + (r.includes('/workspace') ? 'OK' : 'FAIL'));
  } catch (e) { print('[exec cwd] ERROR: ' + e.message); }

  // ── 4. exec (sync) — environment variable ────────────────────────────────
  print('\n[4] exec (sync): custom env var');
  try {
    var r = await skills.exec('echo $MY_VAR', { env: { MY_VAR: 'hello_openclaw' } });
    logResult('exec (env)', r);
    print('    Env var injected: ' + (r.includes('hello_openclaw') ? 'OK' : 'FAIL'));
  } catch (e) { print('[exec env] ERROR: ' + e.message); }

  // ── 5. exec (background) — long-running process ───────────────────────────
  print('\n[5] exec (background): sleep 3 in background');
  var bgSessionId = null;
  try {
    var r = await skills.execBackground('sleep 3 && echo "bg_done"');
    logResult('exec (background)', r);
    // Response is JSON like: {"sessionId":"proc-abc123","status":"running"}
    try {
      var parsed = JSON.parse(r);
      bgSessionId = parsed.sessionId;
      print('    Session ID: ' + bgSessionId);
    } catch (e) {
      // Some impls return the sessionId directly in the string
      var m = r.match(/sessionId["\s:]+([a-z0-9_-]+)/i);
      if (m) bgSessionId = m[1];
    }
    print('    Got session: ' + (bgSessionId ? 'OK' : 'no sessionId in: ' + r));
  } catch (e) { print('[exec background] ERROR: ' + e.message); }

  // ── 6. process.list — list background sessions ───────────────────────────
  print('\n[6] process.list');
  try {
    var r = await skills.processList();
    logResult('process.list', r);
  } catch (e) { print('[process.list] ERROR: ' + e.message); }

  // ── 7. process.poll — poll a session ──────────────────────────────────────
  if (bgSessionId) {
    print('\n[7] process.poll: poll session ' + bgSessionId);
    try {
      var r = await skills.processPoll(bgSessionId);
      logResult('process.poll', r);
    } catch (e) { print('[process.poll] ERROR: ' + e.message); }

    // ── 8. process.log — read stdout lines ─────────────────────────────────
    print('\n[8] process.log: read first 10 lines from session ' + bgSessionId);
    try {
      var r = await skills.processLog(bgSessionId, 0, 10);
      logResult('process.log', r);
    } catch (e) { print('[process.log] ERROR: ' + e.message); }

    // ── 9. process.kill — kill the background session ──────────────────────
    print('\n[9] process.kill: kill session ' + bgSessionId);
    try {
      var r = await skills.processKill(bgSessionId);
      logResult('process.kill', r);
    } catch (e) { print('[process.kill] ERROR: ' + e.message); }
  } else {
    print('\n[7-9] Skipping poll/log/kill — no background session ID captured');
  }

  // ── 10. process.poll — unknown session (graceful error) ───────────────────
  print('\n[10] process.poll: unknown session (expect graceful error)');
  try {
    var r = await skills.processPoll('no-such-session-openclaw-test');
    logResult('process.poll (unknown)', r);
    print('    Graceful: ' + (r.includes('not found') || r.includes('unknown') ? 'OK' : r));
  } catch (e) { print('[process.poll unknown] ERROR: ' + e.message); }

  // ── 11. process.clear — remove completed sessions ─────────────────────────
  print('\n[11] process.clear');
  try {
    var r = await skills.processClear();
    logResult('process.clear', r);
  } catch (e) { print('[process.clear] ERROR: ' + e.message); }

  // ── Summary ───────────────────────────────────────────────────────────────
  print('\n[exec_skills] === All Exec / Process Skills Demo Complete ===');

  try {
    await writeLocalFile('/workspace/exec_skills_result.txt',
      'OpenClaw Exec / Process Skills Demo\n' +
      'Completed: ' + new Date().toISOString() + '\n' +
      'Skills tested: exec (sync/bg/cwd/env), process.list/poll/log/kill/clear\n'
    );
    print('[exec_skills] Result written to /workspace/exec_skills_result.txt');
  } catch (e) { print('[writeLocalFile] ERROR: ' + e.message); }
}

main().catch(function(e) { print('[FATAL] ' + e.message); });
