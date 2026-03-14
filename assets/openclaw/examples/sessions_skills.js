/**
 * OpenClaw Sessions / Agents Skills — WasmEdge QuickJS Example
 *
 * Skills demonstrated:
 *   sessions.list, sessions.history, sessions.send, sessions.spawn,
 *   session.status, agents.list,
 *   gateway.restart, gateway.config.get, gateway.config.schema,
 *   gateway.config.patch, gateway.update.run
 */

import { SkillClient, writeLocalFile, logResult } from '../sdk/skills.js';

async function main() {
  print('[sessions_skills] === OpenClaw Sessions / Gateway Skills Demo ===');

  var skills = new SkillClient({ gatewayUrl: 'http://127.0.0.1:7878' });

  // ── 1. agents.list — list all registered agents ───────────────────────────
  print('\n[1] agents.list');
  try {
    var r = await skills.agentsList();
    logResult('agents.list', r);
    print('    Graceful response: ' + (r.length > 0 ? 'OK' : 'EMPTY (no agents registered)'));
  } catch (e) { print('[agents.list] ERROR: ' + e.message); }

  // ── 2. sessions.list — list active sessions ───────────────────────────────
  print('\n[2] sessions.list (limit=10)');
  try {
    var r = await skills.sessionsList(10);
    logResult('sessions.list', r);
  } catch (e) { print('[sessions.list] ERROR: ' + e.message); }

  // ── 3. sessions.spawn — spawn a new agent session ────────────────────────
  print('\n[3] sessions.spawn: spawn code-reviewer-001');
  var spawnedSessionId = null;
  try {
    var r = await skills.sessionsSpawn(
      'code-reviewer-001',
      'Review the latest pull request for security issues'
    );
    logResult('sessions.spawn', r);
    // Try to parse session ID from response
    try {
      var parsed = JSON.parse(r);
      spawnedSessionId = parsed.sessionId || parsed.id;
    } catch (_) {}
    if (!spawnedSessionId) {
      var m = r.match(/"sessionId"\s*:\s*"([^"]+)"/);
      if (m) spawnedSessionId = m[1];
    }
    print('    Session spawned: ' + (spawnedSessionId || 'ID not parsed from: ' + r.slice(0, 80)));
  } catch (e) { print('[sessions.spawn] ERROR: ' + e.message); }

  // ── 4. session.status — get status of spawned session ────────────────────
  print('\n[4] session.status');
  var testSessionId = spawnedSessionId || 'test-session-openclaw-demo';
  try {
    var r = await skills.sessionStatus(testSessionId);
    logResult('session.status (' + testSessionId + ')', r);
  } catch (e) { print('[session.status] ERROR: ' + e.message); }

  // ── 5. sessions.send — send a message into a session ─────────────────────
  print('\n[5] sessions.send: send message to session');
  try {
    var r = await skills.sessionsSend(
      testSessionId,
      'Please focus on SQL injection vulnerabilities in the auth module.'
    );
    logResult('sessions.send', r);
  } catch (e) { print('[sessions.send] ERROR: ' + e.message); }

  // ── 6. sessions.history — fetch message history ──────────────────────────
  print('\n[6] sessions.history: fetch last 5 messages');
  try {
    var r = await skills.sessionsHistory(testSessionId, 5);
    logResult('sessions.history', r);
  } catch (e) { print('[sessions.history] ERROR: ' + e.message); }

  // ── 7. sessions.history — missing sessionId (graceful) ───────────────────
  print('\n[7] sessions.history: missing sessionId (expect graceful error)');
  try {
    var r = await skills.sessionsHistory(undefined, 10);
    logResult('sessions.history (no id)', r);
    print('    Graceful: ' + (r.includes("missing 'sessionId'") ? 'OK' : r.slice(0, 60)));
  } catch (e) { print('[sessions.history no-id] ' + e.message); }

  // ── 8. sessions.send — missing sessionId (graceful) ──────────────────────
  print('\n[8] sessions.send: missing sessionId (expect graceful error)');
  try {
    var r = await skills.sessionsSend(undefined, 'hello');
    logResult('sessions.send (no id)', r);
    print('    Graceful: ' + (r.includes("missing 'sessionId'") ? 'OK' : r.slice(0, 60)));
  } catch (e) { print('[sessions.send no-id] ' + e.message); }

  // ── 9. gateway.config.get ─────────────────────────────────────────────────
  print('\n[9] gateway.config.get');
  try {
    var r = await skills.gatewayConfigGet();
    logResult('gateway.config.get', r);
  } catch (e) { print('[gateway.config.get] ERROR: ' + e.message); }

  // ── 10. gateway.config.schema ────────────────────────────────────────────
  print('\n[10] gateway.config.schema');
  try {
    var r = await skills.gatewayConfigSchema();
    logResult('gateway.config.schema', r);
  } catch (e) { print('[gateway.config.schema] ERROR: ' + e.message); }

  // ── 11. gateway.config.patch ──────────────────────────────────────────────
  print('\n[11] gateway.config.patch: set logLevel=debug');
  try {
    var r = await skills.gatewayConfigPatch({ logLevel: 'debug' });
    logResult('gateway.config.patch', r);
  } catch (e) { print('[gateway.config.patch] ERROR: ' + e.message); }

  // ── 12. gateway.update.run ────────────────────────────────────────────────
  print('\n[12] gateway.update.run');
  try {
    var r = await skills.gatewayUpdateRun();
    logResult('gateway.update.run', r);
  } catch (e) { print('[gateway.update.run] ERROR: ' + e.message); }

  // ── 13. gateway.restart (last — would restart gateway) ───────────────────
  print('\n[13] gateway.restart (with 0ms delay — graceful if unreachable)');
  try {
    var r = await skills.gatewayRestart(0);
    logResult('gateway.restart', r);
  } catch (e) { print('[gateway.restart] ERROR: ' + e.message); }

  // ── Summary ───────────────────────────────────────────────────────────────
  print('\n[sessions_skills] === All Sessions / Gateway Skills Demo Complete ===');

  try {
    await writeLocalFile('/workspace/sessions_skills_result.txt',
      'OpenClaw Sessions / Gateway Skills Demo\n' +
      'Completed: ' + new Date().toISOString() + '\n' +
      'Skills tested: agents.list, sessions.list, sessions.spawn,\n' +
      '               session.status, sessions.send, sessions.history,\n' +
      '               gateway.config.get, gateway.config.schema,\n' +
      '               gateway.config.patch, gateway.update.run, gateway.restart\n' +
      'Test session ID: ' + testSessionId + '\n'
    );
    print('[sessions_skills] Result written to /workspace/sessions_skills_result.txt');
  } catch (e) { print('[writeLocalFile] ERROR: ' + e.message); }
}

main().catch(function(e) { print('[FATAL] ' + e.message); });
