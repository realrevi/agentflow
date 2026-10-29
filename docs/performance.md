# Performance

A light check for phase 11 (UI polish), before and after, on the same Windows 11 machine.

## How it was measured

- **Build:** `pnpm build`, then in `src-tauri`
  `CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true cargo build --release --features tauri/custom-protocol`
  (a release binary with the production frontend embedded, no bundling; debug assertions
  only so that `AGENTFLOW_DATA_DIR` points it at a throwaway data folder).
- **Scene:** a small demo workspace, onboarding done, three panels: a shell (terminal group),
  Files (right) and the Agents library (center).
- **Cold start to interactive:** process start until the status bar text is painted
  (`PrintWindow` polling every 40 ms).
- **Idle RAM:** 20 s after opening the third panel, summed over the app process and its whole
  tree (WebView2 browser, GPU, renderer and utility processes, the shell's conhost and
  PowerShell). Working set and private bytes.
- **Idle CPU:** CPU time of that process tree over the next 10 s, as a share of all cores.
- **JS size:** every `.js` file in `dist/assets` (raw and gzip); "initial" is what
  `index.html` loads up front, everything else is lazy.

## Results

| Metric                         | Before (phase 10) | After (phase 11) |
| ------------------------------ | ----------------- | ---------------- |
| Cold start to interactive      | 1249–1392 ms      | 1215–1514 ms     |
| Idle RAM, working set (tree)   | 493–511 MB        | 515–535 MB       |
| Idle RAM, private bytes (tree) | 262–281 MB        | 300–304 MB       |
| Idle CPU (10 s)                | 0.01–0.07 %       | 0.03–0.17 %      |
| Total JS (raw / gzip)          | 2429 KB / 708 KB  | 2561 KB / 747 KB |
| Initial JS (raw / gzip)        | 570 KB / 155 KB   | 651 KB / 179 KB  |

Three to five runs each. The start time and idle CPU stay within run-to-run noise. The idle
CPU readings are a few tens of milliseconds over 10 s (the shell's PowerShell included), so
single runs jump around.

## Notes

- **RAM (+~25 MB private):** split by process, about +15 MB is in the WebView2 GPU process and
  +7 MB in the renderer; the app process itself is unchanged. Nothing animates at idle in the
  scene (no avatars on screen, dismissing the tips card changes nothing), so this is raster
  and glyph caches for the new UI font (Inter instead of the system font) and the extra icon
  content, not a running loop. Accepted for now; worth another look in a later phase.
- **Initial JS (+81 KB, +24 KB gzip):** the Lucide subset (72 icons as data) and the shared
  components (Button, fields, tabs, dialog) now load with the shell. marked + DOMPurify,
  CodeMirror, xterm.js and all panels stay lazy.
- **Fixed during the check:** idle avatars only breathe at scale 2 and up (not in tabs and
  lists), and the breathing (a CSS animation, outside the shared loop, which stays stopped
  at idle) pauses while the window is hidden or unfocused.

## Phase 12: font trim

Same method and build flags, a fresh scene (new data folder, same three panels), the phase 11
build ("base") against the trimmed one, interleaved, three runs each:

| Metric                         | Base (phase 11) | Font trim    |
| ------------------------------ | --------------- | ------------ |
| Idle RAM, private bytes (tree) | 245–296 MB      | 242–313 MB   |
| Idle RAM, working set (tree)   | 457–530 MB      | 459–531 MB   |
| WebView2 GPU process (private) | 113–121 MB      | 116–118 MB   |
| WebView2 renderer (private)    | 51.5–51.6 MB    | 51.3–51.9 MB |
| Bundled Inter font files       | 152 KB          | 64 KB        |

- Inter is now subset to the 400–700 weight range and the Latin Extended letters we need
  (fontTools). That saves 88 KB on disk but does not move RAM: the difference is well inside
  run-to-run noise, which is ±30 MB for the whole tree.
- File and brand icons were already loaded on first use (file icons are `<img>` URLs; brand
  icons come with the lazy markdown chunk), and all fonts load per `unicode-range` subset.
- The ~25 MB phase 11 increase stays: it sits in WebView2's own GPU and renderer processes.
  Lowering it would mean WebView2 browser flags (for example turning off GPU rasterization),
  a trade against smoothness and battery that is out of scope for this release.
- This round's cold-start numbers are not comparable: the status-bar probe reported ~7.5 s
  for both builds, while screenshots showed the window ready within 2 s.
