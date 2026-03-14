/**
 * OpenClaw Agent Memory / Knowledge Skills — WasmEdge QuickJS Example
 *
 * Skills demonstrated:
 *   agent.getMemory, agent.setMemory, agent.clearMemory,
 *   agent.listSkills, agent.getContext, agent.delegate,
 *   knowledge.query, knowledge.retrieve,
 *   security.getStatus, security.listEvents,
 *   loop_detection.status, loop_detection.reset,
 *   image (analysis stub)
 */

import { SkillClient, writeLocalFile, logResult } from '../sdk/skills.js';

async function main() {
  print('[agent_skills] === OpenClaw Agent / Knowledge Skills Demo ===');

  var skills = new SkillClient({ gatewayUrl: 'http://127.0.0.1:7878' });

  // ── 1. agent.listSkills — enumerate all grantable skills ─────────────────
  print('\n[1] agent.listSkills');
  try {
    var r = await skills.agentListSkills();
    var arr = JSON.parse(r);
    print('    Total grantable skills: ' + arr.length);
    print('    First 5: ' + arr.slice(0, 5).join(', '));
    logResult('agent.listSkills', 'OK — ' + arr.length + ' skills');
  } catch (e) { print('[agent.listSkills] ERROR: ' + e.message); }

  // ── 2. agent.setMemory — store key-value pairs ───────────────────────────
  print('\n[2] agent.setMemory: set multiple keys');
  var memKeys = [
    { key: 'project_name',  value: 'OpenClaw+ WasmEdge Demo' },
    { key: 'author',        value: 'OpenClaw Agent' },
    { key: 'rust_tip',      value: 'Use cargo clippy for linting' },
    { key: 'wasm_api',      value: 'wasi_net provides TLS connections in QuickJS' },
  ];
  for (var i = 0; i < memKeys.length; i++) {
    try {
      var r = await skills.agentSetMemory(memKeys[i].key, memKeys[i].value);
      print('    Set "' + memKeys[i].key + '": ' + (r.includes('set') ? 'OK' : r));
    } catch (e) { print('[agent.setMemory] ERROR: ' + e.message); }
  }

  // ── 3. agent.getMemory — retrieve a specific key ─────────────────────────
  print('\n[3] agent.getMemory: get "project_name"');
  try {
    var r = await skills.agentGetMemory('project_name');
    logResult('agent.getMemory (project_name)', r);
    print('    Correct value: ' + (r.includes('OpenClaw') ? 'OK' : 'FAIL: ' + r));
  } catch (e) { print('[agent.getMemory] ERROR: ' + e.message); }

  // ── 4. agent.getMemory — list all keys ───────────────────────────────────
  print('\n[4] agent.getMemory: list all keys (empty key)');
  try {
    var r = await skills.agentGetMemory('');
    logResult('agent.getMemory (all)', r);
  } catch (e) { print('[agent.getMemory all] ERROR: ' + e.message); }

  // ── 5. agent.getContext — task context summary ───────────────────────────
  print('\n[5] agent.getContext');
  try {
    var r = await skills.agentGetContext();
    logResult('agent.getContext', r);
  } catch (e) { print('[agent.getContext] ERROR: ' + e.message); }

  // ── 6. knowledge.query — RAG query (falls back to agent memory) ──────────
  print('\n[6] knowledge.query: "rust" (should find rust_tip in memory)');
  try {
    var r = await skills.knowledgeQuery('rust');
    logResult('knowledge.query (rust)', r);
    print('    Hit memory: ' + (r.includes('Found in agent memory') ? 'YES' : 'NO — using stub'));
  } catch (e) { print('[knowledge.query] ERROR: ' + e.message); }

  // ── 7. knowledge.query — no match (returns stub message) ─────────────────
  print('\n[7] knowledge.query: "xyzzy_no_match" (stub message expected)');
  try {
    var r = await skills.knowledgeQuery('xyzzy_no_match_openclaw_test');
    logResult('knowledge.query (no match)', r);
    print('    Got stub hint: ' + (r.includes('not configured') ? 'YES' : 'NO'));
  } catch (e) { print('[knowledge.query] ERROR: ' + e.message); }

  // ── 8. knowledge.retrieve — alias ────────────────────────────────────────
  print('\n[8] knowledge.retrieve: "wasm api"');
  try {
    var r = await skills.knowledgeRetrieve('wasm api');
    logResult('knowledge.retrieve', r);
    print('    Hit memory: ' + (r.includes('wasi_net') || r.includes('agent memory') ? 'YES' : 'NO'));
  } catch (e) { print('[knowledge.retrieve] ERROR: ' + e.message); }

  // ── 9. agent.clearMemory — wipe all memory ───────────────────────────────
  print('\n[9] agent.clearMemory');
  try {
    var r = await skills.agentClearMemory();
    logResult('agent.clearMemory', r);
    // Verify: should get "not found" for previous key
    var r2 = await skills.agentGetMemory('project_name');
    print('    Memory cleared: ' + (r2.includes('not found') ? 'OK' : 'FAIL: ' + r2));
  } catch (e) { print('[agent.clearMemory] ERROR: ' + e.message); }

  // ── 10. agent.delegate — delegate sub-task to another agent ──────────────
  print('\n[10] agent.delegate: delegate to "summarizer-agent" (gateway unreachable is OK)');
  try {
    var r = await skills.agentDelegate(
      'summarizer-agent',
      'Summarize the latest quarterly report',
      30
    );
    logResult('agent.delegate', r);
    print('    Graceful (unreachable OK): ' + (r.includes('gateway') || r.includes('unreachable') || r.includes('Delegated') ? 'YES' : r));
  } catch (e) { print('[agent.delegate] ERROR: ' + e.message); }

  // ── 11. security.getStatus ────────────────────────────────────────────────
  print('\n[11] security.getStatus');
  try {
    var r = await skills.securityGetStatus();
    logResult('security.getStatus', r);
  } catch (e) { print('[security.getStatus] ERROR: ' + e.message); }

  // ── 12. security.listEvents ───────────────────────────────────────────────
  print('\n[12] security.listEvents (limit=5)');
  try {
    var r = await skills.securityListEvents(5);
    logResult('security.listEvents', r);
  } catch (e) { print('[security.listEvents] ERROR: ' + e.message); }

  // ── 13. loop_detection.status ────────────────────────────────────────────
  print('\n[13] loop_detection.status');
  try {
    var r = await skills.loopDetectionStatus();
    logResult('loop_detection.status', r);
  } catch (e) { print('[loop_detection.status] ERROR: ' + e.message); }

  // ── 14. loop_detection.reset ─────────────────────────────────────────────
  print('\n[14] loop_detection.reset');
  try {
    var r = await skills.loopDetectionReset();
    logResult('loop_detection.reset', r);
  } catch (e) { print('[loop_detection.reset] ERROR: ' + e.message); }

  // ── 15. image — vision analysis stub ─────────────────────────────────────
  print('\n[15] image: analyze local file (stub if vision model not configured)');
  try {
    var r = await skills.imageAnalyze(
      '/workspace/demo/screenshot.png',
      'Describe the UI layout shown in this screenshot.'
    );
    logResult('image', r);
  } catch (e) {
    // Expected: file not found or model not configured
    print('[image] ' + (e.message.includes('missing') || e.message.includes('404') ? 'Stub OK (no file/model)' : 'ERROR: ' + e.message));
  }

  // ── Summary ───────────────────────────────────────────────────────────────
  print('\n[agent_skills] === All Agent / Knowledge Skills Demo Complete ===');

  try {
    await writeLocalFile('/workspace/agent_skills_result.txt',
      'OpenClaw Agent / Knowledge Skills Demo\n' +
      'Completed: ' + new Date().toISOString() + '\n' +
      'Skills tested: agent.listSkills, agent.setMemory, agent.getMemory,\n' +
      '               agent.clearMemory, agent.getContext, agent.delegate,\n' +
      '               knowledge.query, knowledge.retrieve,\n' +
      '               security.getStatus, security.listEvents,\n' +
      '               loop_detection.status, loop_detection.reset,\n' +
      '               image\n'
    );
    print('[agent_skills] Result written to /workspace/agent_skills_result.txt');
  } catch (e) { print('[writeLocalFile] ERROR: ' + e.message); }
}

main().catch(function(e) { print('[FATAL] ' + e.message); });
