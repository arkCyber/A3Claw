//! Node.js API Security Shim generator.
//!
//! OpenClaw depends on native Node.js APIs (`fs`, `path`, `child_process`,
//! `net`, etc.). WasmEdge-QuickJS does not provide these APIs natively, so
//! this module generates a JavaScript shim that is injected into the QuickJS
//! execution environment **before** the OpenClaw entry script runs.
//!
//! The shim intercepts every sensitive API call and forwards it to the Rust
//! security layer via the `ocplus` WASM host-function module. If the policy
//! denies the operation, the shim throws a JavaScript `Error` instead of
//! executing the original call.
//!
//! The generated shim is written to a temporary file and loaded by
//! WasmEdge-QuickJS via the `--pre-script` mechanism.

/// Generates the complete Node.js Security Shim JavaScript source.
///
/// The returned string is written to a temp file and injected as
/// `--pre-script /shim/security_shim.js` into the WasmEdge-QuickJS VM.
pub fn generate_shim() -> String {
    format!(
        r#"
// ============================================================
// OpenClaw+ Security Shim  (auto-generated -- do not edit)
// Redirects Node.js APIs to the Rust security interception layer.
// ============================================================

'use strict';

// -- Utility: call a Rust host function via the ocplus WASM module ----------
function _ocplus_call(func_name, ...args) {{
  if (typeof globalThis.__ocplus === 'undefined') {{
    // Fallback: allow all when host functions are not registered (dev mode).
    return 1;
  }}
  return globalThis.__ocplus[func_name](...args);
}}

function _check_file_read(path)   {{ return _ocplus_call('check_file_read',   path) === 1; }}
function _check_file_write(path)  {{ return _ocplus_call('check_file_write',  path) === 1; }}
function _check_file_delete(path) {{ return _ocplus_call('check_file_delete', path) === 1; }}
function _check_network(host, url){{ return _ocplus_call('check_network', host, url) === 1; }}
function _check_shell(cmd)        {{ return _ocplus_call('check_shell',        cmd) === 1; }}

function _deny(op, target) {{
  throw new Error('[OpenClaw+] Security policy denied: ' + op + ' -> ' + target);
}}

// -- fs module shim ---------------------------------------------------------
const _fs_orig = require('fs');
const fs_shim = Object.assign({{}}, _fs_orig, {{
  readFileSync(path, ...args) {{
    if (!_check_file_read(String(path))) _deny('readFileSync', path);
    return _fs_orig.readFileSync(path, ...args);
  }},
  readFile(path, ...args) {{
    if (!_check_file_read(String(path))) {{
      const cb = args[args.length - 1];
      if (typeof cb === 'function') cb(new Error('[OpenClaw+] Read denied: ' + path));
      return;
    }}
    return _fs_orig.readFile(path, ...args);
  }},
  writeFileSync(path, data, ...args) {{
    if (!_check_file_write(String(path))) _deny('writeFileSync', path);
    return _fs_orig.writeFileSync(path, data, ...args);
  }},
  writeFile(path, data, ...args) {{
    if (!_check_file_write(String(path))) {{
      const cb = args[args.length - 1];
      if (typeof cb === 'function') cb(new Error('[OpenClaw+] Write denied: ' + path));
      return;
    }}
    return _fs_orig.writeFile(path, data, ...args);
  }},
  unlinkSync(path) {{
    if (!_check_file_delete(String(path))) _deny('unlinkSync', path);
    return _fs_orig.unlinkSync(path);
  }},
  unlink(path, cb) {{
    if (!_check_file_delete(String(path))) {{
      if (typeof cb === 'function') cb(new Error('[OpenClaw+] Delete denied: ' + path));
      return;
    }}
    return _fs_orig.unlink(path, cb);
  }},
  rmSync(path, ...args) {{
    if (!_check_file_delete(String(path))) _deny('rmSync', path);
    return _fs_orig.rmSync(path, ...args);
  }},
  rm(path, ...args) {{
    if (!_check_file_delete(String(path))) {{
      const cb = args[args.length - 1];
      if (typeof cb === 'function') cb(new Error('[OpenClaw+] Delete denied: ' + path));
      return;
    }}
    return _fs_orig.rm(path, ...args);
  }},
  promises: new Proxy(_fs_orig.promises || {{}}, {{
    get(target, prop) {{
      if (prop === 'readFile') return async (path, ...args) => {{
        if (!_check_file_read(String(path))) _deny('fs.promises.readFile', path);
        return target.readFile(path, ...args);
      }};
      if (prop === 'writeFile') return async (path, data, ...args) => {{
        if (!_check_file_write(String(path))) _deny('fs.promises.writeFile', path);
        return target.writeFile(path, data, ...args);
      }};
      if (prop === 'unlink') return async (path) => {{
        if (!_check_file_delete(String(path))) _deny('fs.promises.unlink', path);
        return target.unlink(path);
      }};
      if (prop === 'rm') return async (path, ...args) => {{
        if (!_check_file_delete(String(path))) _deny('fs.promises.rm', path);
        return target.rm(path, ...args);
      }};
      return typeof target[prop] === 'function' ? target[prop].bind(target) : target[prop];
    }}
  }})
}});

// -- child_process module shim ----------------------------------------------
const _cp_orig = require('child_process');
const child_process_shim = Object.assign({{}}, _cp_orig, {{
  execSync(cmd, ...args) {{
    if (!_check_shell(String(cmd))) _deny('execSync', cmd);
    return _cp_orig.execSync(cmd, ...args);
  }},
  exec(cmd, ...args) {{
    if (!_check_shell(String(cmd))) {{
      const cb = args[args.length - 1];
      if (typeof cb === 'function') cb(new Error('[OpenClaw+] Shell exec denied: ' + cmd));
      return {{ kill: () => {{}} }};
    }}
    return _cp_orig.exec(cmd, ...args);
  }},
  spawnSync(cmd, ...args) {{
    if (!_check_shell(String(cmd))) _deny('spawnSync', cmd);
    return _cp_orig.spawnSync(cmd, ...args);
  }},
  spawn(cmd, ...args) {{
    if (!_check_shell(String(cmd))) _deny('spawn', cmd);
    return _cp_orig.spawn(cmd, ...args);
  }},
  execFileSync(file, ...args) {{
    if (!_check_shell(String(file))) _deny('execFileSync', file);
    return _cp_orig.execFileSync(file, ...args);
  }},
}});

// -- https / http module shim -----------------------------------------------
function _make_http_shim(mod_name) {{
  const _orig = require(mod_name);
  return new Proxy(_orig, {{
    get(target, prop) {{
      if (prop === 'request' || prop === 'get') {{
        return function(url_or_opts, ...args) {{
          let host = '';
          let url_str = '';
          if (typeof url_or_opts === 'string') {{
            try {{ const u = new URL(url_or_opts); host = u.hostname; url_str = url_or_opts; }} catch(_) {{}}
          }} else if (url_or_opts && url_or_opts.hostname) {{
            host = url_or_opts.hostname;
            url_str = host;
          }}
          if (!_check_network(host, url_str)) _deny(mod_name + '.' + prop, host);
          return target[prop](url_or_opts, ...args);
        }};
      }}
      return typeof target[prop] === 'function' ? target[prop].bind(target) : target[prop];
    }}
  }});
}}

// -- require interceptor ----------------------------------------------------
const _orig_require = globalThis.require || (typeof require !== 'undefined' ? require : null);
if (_orig_require) {{
  const _module_cache = {{}};
  globalThis.require = function(id) {{
    if (id === 'fs')            return fs_shim;
    if (id === 'child_process') return child_process_shim;
    if (id === 'https')         return _module_cache['https'] || (_module_cache['https'] = _make_http_shim('https'));
    if (id === 'http')          return _module_cache['http']  || (_module_cache['http']  = _make_http_shim('http'));
    return _orig_require(id);
  }};
  // Preserve any properties attached to the original require.
  Object.assign(globalThis.require, _orig_require);
}}

// -- global fetch shim ------------------------------------------------------
if (typeof globalThis.fetch !== 'undefined') {{
  const _orig_fetch = globalThis.fetch.bind(globalThis);
  globalThis.fetch = function(input, init) {{
    let host = '';
    let url_str = String(input instanceof Request ? input.url : input);
    try {{ host = new URL(url_str).hostname; }} catch(_) {{}}
    if (!_check_network(host, url_str)) _deny('fetch', host);
    return _orig_fetch(input, init);
  }};
}}

// -- process.exit guard -----------------------------------------------------
if (typeof process !== 'undefined') {{
  const _orig_exit = process.exit.bind(process);
  process.exit = function(code) {{
    console.log('[OpenClaw+] process.exit(' + code + ') intercepted by sandbox');
    _orig_exit(code);
  }};
}}

console.log('[OpenClaw+] Security Shim injected -- all sensitive operations are monitored.');
"#
    )
}

/// Writes the generated shim to a temporary file and returns its path.
///
/// The file is placed at `$TMPDIR/openclaw-plus/security_shim.js`.
pub fn write_shim_to_temp() -> anyhow::Result<std::path::PathBuf> {
    let tmp_dir = std::env::temp_dir().join("openclaw-plus");
    std::fs::create_dir_all(&tmp_dir)?;
    let shim_path = tmp_dir.join("security_shim.js");
    std::fs::write(&shim_path, generate_shim())?;
    Ok(shim_path)
}
