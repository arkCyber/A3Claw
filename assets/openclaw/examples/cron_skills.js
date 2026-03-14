/**
 * OpenClaw Cron Scheduler Skills — WasmEdge QuickJS Example
 *
 * Skills demonstrated:
 *   cron.status, cron.list, cron.add, cron.remove, cron.run
 */

import { SkillClient, writeLocalFile, logResult } from '../sdk/skills.js';

async function main() {
  print('[cron_skills] === OpenClaw Cron Scheduler Skills Demo ===');

  var skills = new SkillClient({ gatewayUrl: 'http://127.0.0.1:7878' });

  // ── 1. cron.status ───────────────────────────────────────────────────────
  print('\n[1] cron.status');
  try {
    var r = await skills.cronStatus();
    logResult('cron.status', r);
    print('    Has status field: ' + (r.includes('status') || r.includes('jobCount') ? 'OK' : r));
  } catch (e) { print('[cron.status] ERROR: ' + e.message); }

  // ── 2. cron.list (empty at start) ────────────────────────────────────────
  print('\n[2] cron.list (initially empty)');
  try {
    var r = await skills.cronList();
    logResult('cron.list (empty)', r);
    var jobs = JSON.parse(r);
    print('    Jobs initially: ' + jobs.length);
  } catch (e) { print('[cron.list] ' + e.message); }

  // ── 3. cron.add — daily report job ───────────────────────────────────────
  print('\n[3] cron.add: daily-report at 09:00');
  var dailyJobId = null;
  try {
    var r = await skills.cronAdd(
      'daily-report',
      '0 9 * * *',
      'Generate daily sales summary and write to /workspace/reports/daily.txt'
    );
    logResult('cron.add (daily)', r);
    var parsed = JSON.parse(r);
    dailyJobId = parsed.id;
    print('    Job ID: ' + dailyJobId);
  } catch (e) { print('[cron.add daily] ERROR: ' + e.message); }

  // ── 4. cron.add — hourly cleanup job ─────────────────────────────────────
  print('\n[4] cron.add: hourly-cleanup every hour');
  var hourlyJobId = null;
  try {
    var r = await skills.cronAdd(
      'hourly-cleanup',
      '0 * * * *',
      'Remove temporary files from /workspace/tmp',
      true
    );
    logResult('cron.add (hourly)', r);
    var parsed = JSON.parse(r);
    hourlyJobId = parsed.id;
    print('    Job ID: ' + hourlyJobId);
  } catch (e) { print('[cron.add hourly] ERROR: ' + e.message); }

  // ── 5. cron.add — weekly summary job ─────────────────────────────────────
  print('\n[5] cron.add: weekly-summary every Monday 08:00');
  var weeklyJobId = null;
  try {
    var r = await skills.cronAdd(
      'weekly-summary',
      '0 8 * * 1',
      'Compile weekly metrics and send digest to /workspace/reports/weekly.txt'
    );
    logResult('cron.add (weekly)', r);
    var parsed = JSON.parse(r);
    weeklyJobId = parsed.id;
    print('    Job ID: ' + weeklyJobId);
  } catch (e) { print('[cron.add weekly] ERROR: ' + e.message); }

  // ── 6. cron.list — verify all three jobs ─────────────────────────────────
  print('\n[6] cron.list: expect 3 jobs');
  try {
    var r = await skills.cronList();
    logResult('cron.list (after adds)', r);
    var jobs = JSON.parse(r);
    print('    Jobs listed: ' + jobs.length);
    print('    Has daily-report: ' + (r.includes('daily-report') ? 'YES' : 'NO'));
    print('    Has hourly-cleanup: ' + (r.includes('hourly-cleanup') ? 'YES' : 'NO'));
  } catch (e) { print('[cron.list] ERROR: ' + e.message); }

  // ── 7. cron.run — manually trigger daily-report ──────────────────────────
  if (dailyJobId) {
    print('\n[7] cron.run: manually trigger daily-report (' + dailyJobId + ')');
    try {
      var r = await skills.cronRun(dailyJobId);
      logResult('cron.run (daily)', r);
      print('    Triggered: ' + (r.includes('triggered') || r.includes('run') ? 'OK' : r));
    } catch (e) { print('[cron.run] ERROR: ' + e.message); }
  }

  // ── 8. cron.run — manually trigger hourly-cleanup ────────────────────────
  if (hourlyJobId) {
    print('\n[8] cron.run: manually trigger hourly-cleanup (' + hourlyJobId + ')');
    try {
      var r = await skills.cronRun(hourlyJobId);
      logResult('cron.run (hourly)', r);
    } catch (e) { print('[cron.run hourly] ERROR: ' + e.message); }
  }

  // ── 9. cron.run — unknown job ID (expect error) ──────────────────────────
  print('\n[9] cron.run: unknown job ID (expect error)');
  try {
    var r = await skills.cronRun('job-does-not-exist-openclaw-test');
    print('[cron.run unknown] Unexpected OK: ' + r);
  } catch (e) {
    print('[cron.run unknown] Got expected error: ' + e.message.slice(0, 80));
  }

  // ── 10. cron.remove — remove weekly job ──────────────────────────────────
  if (weeklyJobId) {
    print('\n[10] cron.remove: remove weekly-summary (' + weeklyJobId + ')');
    try {
      var r = await skills.cronRemove(weeklyJobId);
      logResult('cron.remove (weekly)', r);
      print('    Removed: ' + (r.includes('removed') || r.includes(weeklyJobId) ? 'OK' : r));
    } catch (e) { print('[cron.remove] ERROR: ' + e.message); }
  }

  // ── 11. cron.list — verify weekly job is gone ────────────────────────────
  print('\n[11] cron.list: verify weekly job removed');
  try {
    var r = await skills.cronList();
    var jobs = JSON.parse(r);
    print('    Jobs remaining: ' + jobs.length);
    print('    weekly-summary gone: ' + (!r.includes('weekly-summary') ? 'YES' : 'NO (still there)'));
    logResult('cron.list (after remove)', jobs.length + ' jobs remain');
  } catch (e) { print('[cron.list final] ERROR: ' + e.message); }

  // ── 12. cron.status — check final state ──────────────────────────────────
  print('\n[12] cron.status: final state');
  try {
    var r = await skills.cronStatus();
    logResult('cron.status (final)', r);
  } catch (e) { print('[cron.status final] ERROR: ' + e.message); }

  // ── Summary ───────────────────────────────────────────────────────────────
  print('\n[cron_skills] === All Cron Skills Demo Complete ===');

  try {
    await writeLocalFile('/workspace/cron_skills_result.txt',
      'OpenClaw Cron Scheduler Skills Demo\n' +
      'Completed: ' + new Date().toISOString() + '\n' +
      'Skills tested: cron.status, cron.list, cron.add (x3), cron.run (x2), cron.remove\n' +
      'Daily job ID:  ' + (dailyJobId || 'N/A') + '\n' +
      'Hourly job ID: ' + (hourlyJobId || 'N/A') + '\n'
    );
    print('[cron_skills] Result written to /workspace/cron_skills_result.txt');
  } catch (e) { print('[writeLocalFile] ERROR: ' + e.message); }
}

main().catch(function(e) { print('[FATAL] ' + e.message); });
