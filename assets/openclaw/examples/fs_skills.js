/**
 * OpenClaw File System Skills — WasmEdge QuickJS Example
 *
 * Demonstrates all 9 fs.* skills + apply_patch:
 *   fs.readFile, fs.readDir, fs.stat, fs.exists,
 *   fs.writeFile, fs.mkdir, fs.deleteFile, fs.move, fs.copy, apply_patch
 *
 * Run inside WasmEdge sandbox with Gateway on http://127.0.0.1:7878.
 *
 * NOTE: This script calls the Gateway /skill/execute endpoint.
 *       For direct local file access (no Gateway) use std.open() from QuickJS.
 */

import { SkillClient, writeLocalFile, logResult } from '../sdk/skills.js';

async function main() {
  print('[fs_skills] === OpenClaw File System Skills Demo ===');

  var skills = new SkillClient({ gatewayUrl: 'http://127.0.0.1:7878' });

  var workspace = '/workspace';

  // ── 1. fs.mkdir — create nested directories ───────────────────────────────
  print('\n[1] fs.mkdir: create /workspace/demo/data');
  try {
    var r = await skills.fsMkdir(workspace + '/demo/data');
    logResult('fs.mkdir', r);
  } catch (e) { print('[fs.mkdir] ERROR: ' + e.message); }

  // ── 2. fs.writeFile — write a text file ───────────────────────────────────
  print('\n[2] fs.writeFile: write hello.txt');
  try {
    var content = 'Hello from OpenClaw WasmEdge Sandbox!\nLine 2: OpenClaw Skills Test\n';
    var r = await skills.fsWriteFile(workspace + '/demo/hello.txt', content);
    logResult('fs.writeFile', r);
  } catch (e) { print('[fs.writeFile] ERROR: ' + e.message); }

  // ── 3. fs.readFile — read back the file ──────────────────────────────────
  print('\n[3] fs.readFile: read hello.txt');
  try {
    var r = await skills.fsReadFile(workspace + '/demo/hello.txt');
    logResult('fs.readFile', r);
    print('    Content verified: ' + (r.includes('OpenClaw') ? 'OK' : 'MISMATCH'));
  } catch (e) { print('[fs.readFile] ERROR: ' + e.message); }

  // ── 4. fs.stat — get file metadata ───────────────────────────────────────
  print('\n[4] fs.stat: stat hello.txt');
  try {
    var r = await skills.fsStat(workspace + '/demo/hello.txt');
    logResult('fs.stat', r);
    print('    Has size info: ' + (r.includes('bytes') ? 'OK' : 'MISSING'));
  } catch (e) { print('[fs.stat] ERROR: ' + e.message); }

  // ── 5. fs.exists — check existence ───────────────────────────────────────
  print('\n[5a] fs.exists: check existing file');
  try {
    var r = await skills.fsExists(workspace + '/demo/hello.txt');
    logResult('fs.exists (existing)', r);
    print('    Expected true: ' + (r === 'true' ? 'OK' : 'FAIL'));
  } catch (e) { print('[fs.exists] ERROR: ' + e.message); }

  print('\n[5b] fs.exists: check missing file');
  try {
    var r = await skills.fsExists(workspace + '/demo/no_such_file_openclaw.txt');
    logResult('fs.exists (missing)', r);
    print('    Expected false: ' + (r === 'false' ? 'OK' : 'FAIL'));
  } catch (e) { print('[fs.exists] ERROR: ' + e.message); }

  // ── 6. fs.writeFile — write second file for copy/move tests ──────────────
  print('\n[6] fs.writeFile: write source_file.txt');
  try {
    await skills.fsWriteFile(workspace + '/demo/source_file.txt', 'Source content for copy/move test\n');
    logResult('fs.writeFile (source)', 'OK');
  } catch (e) { print('[fs.writeFile] ERROR: ' + e.message); }

  // ── 7. fs.copy — copy a file ─────────────────────────────────────────────
  print('\n[7] fs.copy: copy source_file.txt -> copied_file.txt');
  try {
    var r = await skills.fsCopy(
      workspace + '/demo/source_file.txt',
      workspace + '/demo/copied_file.txt'
    );
    logResult('fs.copy', r);
  } catch (e) { print('[fs.copy] ERROR: ' + e.message); }

  // ── 8. fs.move — rename/move a file ──────────────────────────────────────
  print('\n[8] fs.move: move copied_file.txt -> moved_file.txt');
  try {
    var r = await skills.fsMove(
      workspace + '/demo/copied_file.txt',
      workspace + '/demo/moved_file.txt'
    );
    logResult('fs.move', r);
  } catch (e) { print('[fs.move] ERROR: ' + e.message); }

  // ── 9. fs.readDir — list directory ───────────────────────────────────────
  print('\n[9] fs.readDir: list /workspace/demo');
  try {
    var r = await skills.fsReadDir(workspace + '/demo');
    logResult('fs.readDir', r);
    print('    Files listed: ' + r.split('\n').filter(function(l){ return l.trim(); }).length);
  } catch (e) { print('[fs.readDir] ERROR: ' + e.message); }

  // ── 10. apply_patch — search-replace patch ───────────────────────────────
  print('\n[10] apply_patch: search-replace on hello.txt');
  try {
    var patch = (
      '<<<<<<< ' + workspace + '/demo/hello.txt\n' +
      'Hello from OpenClaw WasmEdge Sandbox!\n' +
      '=======\n' +
      'Hello from OpenClaw WasmEdge Sandbox! [PATCHED]\n' +
      '>>>>>>>\n'
    );
    var r = await skills.applyPatch(patch);
    logResult('apply_patch', r);

    // Verify the patch was applied
    var after = await skills.fsReadFile(workspace + '/demo/hello.txt');
    print('    Patch applied: ' + (after.includes('[PATCHED]') ? 'OK' : 'FAIL'));
  } catch (e) { print('[apply_patch] ERROR: ' + e.message); }

  // ── 11. fs.deleteFile — delete a file ────────────────────────────────────
  print('\n[11] fs.deleteFile: delete moved_file.txt');
  try {
    var r = await skills.fsDeleteFile(workspace + '/demo/moved_file.txt');
    logResult('fs.deleteFile', r);

    // Confirm deletion
    var exists = await skills.fsExists(workspace + '/demo/moved_file.txt');
    print('    File gone: ' + (exists === 'false' ? 'OK' : 'FAIL (still exists)'));
  } catch (e) { print('[fs.deleteFile] ERROR: ' + e.message); }

  // ── Summary ───────────────────────────────────────────────────────────────
  print('\n[fs_skills] === All File System Skills Demo Complete ===');

  // Write a local summary using std (no Gateway needed)
  try {
    await writeLocalFile(workspace + '/demo/fs_skills_result.txt',
      'OpenClaw File System Skills Demo\n' +
      'Completed at: ' + new Date().toISOString() + '\n' +
      'Skills tested: fs.mkdir, fs.writeFile, fs.readFile, fs.stat, fs.exists,\n' +
      '               fs.copy, fs.move, fs.readDir, apply_patch, fs.deleteFile\n'
    );
    print('[fs_skills] Result written to ' + workspace + '/demo/fs_skills_result.txt');
  } catch (e) { print('[writeLocalFile] ERROR: ' + e.message); }
}

main().catch(function(e) { print('[FATAL] ' + e.message); });
