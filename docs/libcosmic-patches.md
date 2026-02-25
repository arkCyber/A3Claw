# libcosmic / iced patches for OpenClaw+

These patches are applied directly to the Cargo git checkout at:
`~/.cargo/git/checkouts/libcosmic-41009aea1d72760b/384e8f6/`

They are **not** tracked by this repo's git. After a `cargo clean` or a
fresh checkout you must re-apply them (or use `patch -p1`).

---

## 1. IME multi-character commit fix

**File:** `src/widget/text_input/input.rs`

**Problem:** `text.chars().next()` only inserts the first character of an
IME commit string, so typing "你好" inserts only "你".

**Fix:** collect all non-control characters and insert them one by one.

```diff
-                if let Some(c) = text.and_then(|t| t.chars().next().filter(|c| !c.is_control())) {
-                    ...
-                    editor.insert(c);
+                let printable_text: Option<String> = text.map(|t| {
+                    t.chars().filter(|c| !c.is_control()).collect()
+                }).filter(|s: &String| !s.is_empty());
+                if let Some(printable) = printable_text {
+                    ...
+                    for c in printable.chars() {
+                        editor.insert(c);
+                    }
```

---

## 2. IME enabled on window creation

**File:** `iced/winit/src/program.rs`

**Problem:** `set_ime_allowed(true)` was never called, so macOS routed
CJK input to whatever app owned the terminal.

**Fix (a):** Call `set_ime_allowed(true)` right after `set_visible(true)`
(not on the hidden dummy window).

**Fix (b):** On `WindowEvent::Focused(true)` re-assert IME ownership so
switching back from another app re-binds CJK input to our window.

---

## 3. IME Commit event forwarded to iced

**File:** `iced/winit/src/conversion.rs`

**Problem:** `WindowEvent::Ime(Commit(s))` was silently dropped by
`_ => None`, so committed text never reached the text_input widget.

**Fix:** Convert it to a synthetic `keyboard::Event::KeyPressed` with the
full committed string in the `text` field.

```diff
+        WindowEvent::Ime(winit::event::Ime::Commit(string)) => {
+            if string.is_empty() { return None; }
+            use crate::core::SmolStr;
+            Some(Event::Keyboard(keyboard::Event::KeyPressed {
+                key: keyboard::Key::Unidentified,
+                ...
+                text: Some(SmolStr::new(&string)),
+            }))
+        }
```

---

## 4. IME candidate window position (macOS coordinate mapping)

**File:** `iced/winit/src/program.rs`

**Problem:** Candidate box was appearing over the Dock (screen_y ≈ 17).

**Root cause analysis (confirmed by log):**
- `WinitView.isFlipped = true` → view coords are top-left origin, Y-down
- `firstRectForCharacterRange` calls `convertRect_toView` (flip Y) then
  `convertRectToScreen` (to screen Y-up coords)
- Measured: `view_y=770` in `h=820` → `screen_y=17`
- Formula: `screen_y = (h - view_y) + window_bottom_offset`
  where `window_bottom_offset ≈ -33` for this window

**Fix:** Target `screen_y ≈ 80` (just above input bar, inside window):
```
view_y = h - target_screen_y - |window_bottom_offset|
       = 820 - 80 - 33 = 707
       = h - 113
```

```rust
// On Focused(true):
let ime_y = (logical_size.height as f64 - 113.0).max(0.0);
window.raw.set_ime_cursor_area(
    winit::dpi::Position::Logical(
        winit::dpi::LogicalPosition::new(80.0, ime_y),
    ),
    winit::dpi::Size::Logical(
        winit::dpi::LogicalSize::new(400.0, 28.0),
    ),
);
```

**Verification log:**
```
view_pos=(80,707) view_size=(400,28) => screen=(156,80 400x28)  ✓
```

---

## Re-applying after a clean build

After deleting `target/` or updating the lock file you may need to force
a recompile of the patched crates:

```bash
rm -f target/release/deps/libiced_winit-*.rlib \
      target/release/deps/libiced_winit-*.rmeta \
      target/release/deps/libcosmic-*.rlib \
      target/release/deps/libcosmic-*.rmeta
cargo build --release -p openclaw-ui
```
