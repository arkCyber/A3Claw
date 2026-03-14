/**
 * OpenClaw Messaging / Canvas / Nodes / Email / Calendar Skills
 * WasmEdge QuickJS Example
 *
 * Skills demonstrated:
 *   message.send, message.reply, message.react, message.read,
 *   message.search, message.delete
 *   canvas.create, canvas.get, canvas.update, canvas.list,
 *   canvas.export, canvas.present, canvas.snapshot, canvas.delete
 *   nodes.list, nodes.add, nodes.remove, nodes.connect,
 *   nodes.disconnect, nodes.status, nodes.notify, nodes.run
 *   email.list, email.read, email.send, email.reply, email.delete
 *   calendar.list, calendar.create, calendar.get,
 *   calendar.update, calendar.delete
 *
 * NOTE: email.* and calendar.* require a registered SkillHandler.
 *       The stubs show the expected config hint when not configured.
 *       canvas.* / nodes.* / message.* use Gateway endpoints or stubs.
 */

import { SkillClient, writeLocalFile, logResult } from '../sdk/skills.js';

async function main() {
  print('[messaging_skills] === OpenClaw Messaging / Canvas / Nodes / Email / Calendar Demo ===');

  var skills = new SkillClient({ gatewayUrl: 'http://127.0.0.1:7878' });

  // ══════════════════════════════════════════════════════════════════════════
  // SECTION A: message.* skills
  // ══════════════════════════════════════════════════════════════════════════
  print('\n── Section A: message.* skills ──');

  // A1. message.send
  print('\n[A1] message.send: send to #general on discord');
  try {
    var r = await skills.messageSend('#general', 'Daily status from OpenClaw agent.', 'discord');
    logResult('message.send', r);
  } catch (e) { print('[message.send] ERROR: ' + e.message); }

  // A2. message.send — Slack channel
  print('\n[A2] message.send: send to #alerts on slack');
  try {
    var r = await skills.messageSend('#alerts', 'Build pipeline completed.', 'slack');
    logResult('message.send (slack)', r);
  } catch (e) { print('[message.send slack] ERROR: ' + e.message); }

  // A3. message.reply
  print('\n[A3] message.reply: reply to message m-12345 on discord');
  try {
    var r = await skills.messageReply('m-12345', 'Acknowledged, will follow up.', '#general');
    logResult('message.reply', r);
  } catch (e) { print('[message.reply] ERROR: ' + e.message); }

  // A4. message.react
  print('\n[A4] message.react: add emoji reaction');
  try {
    var r = await skills.messageReact('m-12345', '👍', '#general');
    logResult('message.react', r);
  } catch (e) { print('[message.react] ERROR: ' + e.message); }

  // A5. message.read
  print('\n[A5] message.read: read message m-12345');
  try {
    var r = await skills.messageRead('m-12345', '#general');
    logResult('message.read', r);
  } catch (e) { print('[message.read] ERROR: ' + e.message); }

  // A6. message.search
  print('\n[A6] message.search: search "build pipeline" in #dev');
  try {
    var r = await skills.messageSearch('build pipeline', '#dev');
    logResult('message.search', r);
  } catch (e) { print('[message.search] ERROR: ' + e.message); }

  // A7. message.delete
  print('\n[A7] message.delete: remove message m-99999 from #test');
  try {
    var r = await skills.messageDelete('m-99999', '#test');
    logResult('message.delete', r);
  } catch (e) { print('[message.delete] ERROR: ' + e.message); }

  // A8. message.send — missing channel (verify error path via DispatchFailed)
  print('\n[A8] message.send: missing channel (expect error)');
  try {
    var r = await skills.messageSend(undefined, 'test');
    print('[message.send no-channel] Unexpected OK: ' + r.slice(0, 60));
  } catch (e) {
    print('[message.send no-channel] Got error (OK): ' + e.message.slice(0, 80));
  }

  // ══════════════════════════════════════════════════════════════════════════
  // SECTION B: canvas.* skills
  // ══════════════════════════════════════════════════════════════════════════
  print('\n── Section B: canvas.* skills ──');

  // B1. canvas.create
  print('\n[B1] canvas.create: create "Sprint Board"');
  var canvasId = null;
  try {
    var r = await skills.canvasCreate('Sprint Board', 'kanban');
    logResult('canvas.create', r);
    try {
      var parsed = JSON.parse(r);
      canvasId = parsed.id || parsed.canvasId;
    } catch (_) {}
    print('    Canvas ID: ' + (canvasId || 'not parsed — stub response'));
  } catch (e) { print('[canvas.create] ERROR: ' + e.message); }

  var testCanvasId = canvasId || 'canvas-demo-openclaw';

  // B2. canvas.list
  print('\n[B2] canvas.list');
  try {
    var r = await skills.canvasList();
    logResult('canvas.list', r);
  } catch (e) { print('[canvas.list] ERROR: ' + e.message); }

  // B3. canvas.get
  print('\n[B3] canvas.get: ' + testCanvasId);
  try {
    var r = await skills.canvasGet(testCanvasId);
    logResult('canvas.get', r);
  } catch (e) { print('[canvas.get] ERROR: ' + e.message); }

  // B4. canvas.update
  print('\n[B4] canvas.update: add content to canvas');
  try {
    var r = await skills.canvasUpdate(testCanvasId, '# Sprint Board\n\n- [ ] Task 1\n- [ ] Task 2\n');
    logResult('canvas.update', r);
  } catch (e) { print('[canvas.update] ERROR: ' + e.message); }

  // B5. canvas.export
  print('\n[B5] canvas.export: export as markdown');
  try {
    var r = await skills.canvasExport(testCanvasId, 'markdown');
    logResult('canvas.export', r);
  } catch (e) { print('[canvas.export] ERROR: ' + e.message); }

  // B6. canvas.present
  print('\n[B6] canvas.present: present content to node');
  try {
    var r = await skills.canvasPresent('# Current Sprint Status\n\nAll tasks on track.', 'display-node-1');
    logResult('canvas.present', r);
  } catch (e) { print('[canvas.present] ERROR: ' + e.message); }

  // B7. canvas.snapshot
  print('\n[B7] canvas.snapshot: snapshot display node');
  try {
    var r = await skills.canvasSnapshot('display-node-1');
    logResult('canvas.snapshot', r);
  } catch (e) { print('[canvas.snapshot] ERROR: ' + e.message); }

  // B8. canvas.delete
  print('\n[B8] canvas.delete: ' + testCanvasId);
  try {
    var r = await skills.canvasDelete(testCanvasId);
    logResult('canvas.delete', r);
  } catch (e) { print('[canvas.delete] ERROR: ' + e.message); }

  // ══════════════════════════════════════════════════════════════════════════
  // SECTION C: nodes.* skills
  // ══════════════════════════════════════════════════════════════════════════
  print('\n── Section C: nodes.* skills ──');

  // C1. nodes.status
  print('\n[C1] nodes.status');
  try {
    var r = await skills.nodesStatus();
    logResult('nodes.status', r);
  } catch (e) { print('[nodes.status] ERROR: ' + e.message); }

  // C2. nodes.list
  print('\n[C2] nodes.list');
  try {
    var r = await skills.nodesList();
    logResult('nodes.list', r);
  } catch (e) { print('[nodes.list] ERROR: ' + e.message); }

  // C3. nodes.add
  print('\n[C3] nodes.add: add a model node');
  try {
    var r = await skills.nodesAdd('model', { name: 'qwen2-worker', model: 'qwen2.5:0.5b' });
    logResult('nodes.add', r);
  } catch (e) { print('[nodes.add] ERROR: ' + e.message); }

  // C4. nodes.connect
  print('\n[C4] nodes.connect: node-a -> node-b');
  try {
    var r = await skills.nodesConnect('node-a', 'node-b');
    logResult('nodes.connect', r);
  } catch (e) { print('[nodes.connect] ERROR: ' + e.message); }

  // C5. nodes.notify — send push notification
  print('\n[C5] nodes.notify: send notification to companion app');
  try {
    var r = await skills.nodesNotify(
      'Build Complete',
      'CI pipeline passed all tests.',
      'macbook-companion'
    );
    logResult('nodes.notify', r);
  } catch (e) { print('[nodes.notify] ERROR: ' + e.message); }

  // C6. nodes.run — run command on remote node
  print('\n[C6] nodes.run: run "git status" on build-server');
  try {
    var r = await skills.nodesRun('build-server', 'git status', { cwd: '/workspace' });
    logResult('nodes.run', r);
  } catch (e) { print('[nodes.run] ERROR: ' + e.message); }

  // C7. nodes.disconnect
  print('\n[C7] nodes.disconnect: node-a -> node-b');
  try {
    var r = await skills.nodesDisconnect('node-a', 'node-b');
    logResult('nodes.disconnect', r);
  } catch (e) { print('[nodes.disconnect] ERROR: ' + e.message); }

  // C8. nodes.remove
  print('\n[C8] nodes.remove: qwen2-worker');
  try {
    var r = await skills.nodesRemove('qwen2-worker');
    logResult('nodes.remove', r);
  } catch (e) { print('[nodes.remove] ERROR: ' + e.message); }

  // ══════════════════════════════════════════════════════════════════════════
  // SECTION D: email.* skills (stubs — requires EmailSkillHandler)
  // ══════════════════════════════════════════════════════════════════════════
  print('\n── Section D: email.* skills (stubs) ──');

  // D1. email.list
  print('\n[D1] email.list: INBOX (stub — EmailSkillHandler required)');
  try {
    var r = await skills.emailList({ folder: 'INBOX', limit: 10 });
    logResult('email.list', r);
    print('    Stub hint present: ' + (r.includes('EmailSkillHandler') ? 'YES' : 'NO — handler may be registered'));
  } catch (e) { print('[email.list] ERROR: ' + e.message); }

  // D2. email.read
  print('\n[D2] email.read: read email abc-123');
  try {
    var r = await skills.emailRead('abc-123');
    logResult('email.read', r);
  } catch (e) { print('[email.read] ERROR: ' + e.message); }

  // D3. email.send
  print('\n[D3] email.send: send to user@example.com');
  try {
    var r = await skills.emailSend(
      'user@example.com',
      'OpenClaw Daily Report',
      'Dear user,\n\nYour daily report is ready.\n\nBest,\nOpenClaw Agent'
    );
    logResult('email.send', r);
  } catch (e) { print('[email.send] ERROR: ' + e.message); }

  // D4. email.reply
  print('\n[D4] email.reply: reply to abc-123');
  try {
    var r = await skills.emailReply('abc-123', 'Thank you for your message. Noted.');
    logResult('email.reply', r);
  } catch (e) { print('[email.reply] ERROR: ' + e.message); }

  // D5. email.delete
  print('\n[D5] email.delete: remove email abc-999');
  try {
    var r = await skills.emailDelete('abc-999');
    logResult('email.delete', r);
  } catch (e) { print('[email.delete] ERROR: ' + e.message); }

  // ══════════════════════════════════════════════════════════════════════════
  // SECTION E: calendar.* skills (stubs — requires CalendarSkillHandler)
  // ══════════════════════════════════════════════════════════════════════════
  print('\n── Section E: calendar.* skills (stubs) ──');

  // E1. calendar.list
  print('\n[E1] calendar.list: this week (stub)');
  try {
    var r = await skills.calendarList('2026-03-01', '2026-03-07');
    logResult('calendar.list', r);
    print('    Stub hint: ' + (r.includes('CalendarSkillHandler') ? 'YES' : 'NO'));
  } catch (e) { print('[calendar.list] ERROR: ' + e.message); }

  // E2. calendar.create
  print('\n[E2] calendar.create: create a meeting');
  var eventId = null;
  try {
    var r = await skills.calendarCreate(
      'Sprint Review',
      '2026-03-05T14:00:00Z',
      '2026-03-05T15:00:00Z',
      { description: 'Q1 sprint review meeting', location: 'Zoom' }
    );
    logResult('calendar.create', r);
    try {
      var parsed = JSON.parse(r);
      eventId = parsed.id || parsed.eventId;
    } catch (_) {}
  } catch (e) { print('[calendar.create] ERROR: ' + e.message); }

  var testEventId = eventId || 'event-demo-openclaw';

  // E3. calendar.get
  print('\n[E3] calendar.get: ' + testEventId);
  try {
    var r = await skills.calendarGet(testEventId);
    logResult('calendar.get', r);
  } catch (e) { print('[calendar.get] ERROR: ' + e.message); }

  // E4. calendar.update
  print('\n[E4] calendar.update: change title');
  try {
    var r = await skills.calendarUpdate(testEventId, {
      title: 'Sprint Review (Updated)',
      description: 'Updated: includes demo session'
    });
    logResult('calendar.update', r);
  } catch (e) { print('[calendar.update] ERROR: ' + e.message); }

  // E5. calendar.delete
  print('\n[E5] calendar.delete: ' + testEventId);
  try {
    var r = await skills.calendarDelete(testEventId);
    logResult('calendar.delete', r);
  } catch (e) { print('[calendar.delete] ERROR: ' + e.message); }

  // ── Summary ───────────────────────────────────────────────────────────────
  print('\n[messaging_skills] === All Messaging / Canvas / Nodes / Email / Calendar Demo Complete ===');

  try {
    await writeLocalFile('/workspace/messaging_skills_result.txt',
      'OpenClaw Messaging / Canvas / Nodes / Email / Calendar Demo\n' +
      'Completed: ' + new Date().toISOString() + '\n\n' +
      'Section A — message.*: send, reply, react, read, search, delete\n' +
      'Section B — canvas.*: create, list, get, update, export, present, snapshot, delete\n' +
      'Section C — nodes.*: status, list, add, connect, notify, run, disconnect, remove\n' +
      'Section D — email.* (stubs): list, read, send, reply, delete\n' +
      'Section E — calendar.* (stubs): list, create, get, update, delete\n'
    );
    print('[messaging_skills] Result written to /workspace/messaging_skills_result.txt');
  } catch (e) { print('[writeLocalFile] ERROR: ' + e.message); }
}

main().catch(function(e) { print('[FATAL] ' + e.message); });
