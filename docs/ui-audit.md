# UI audit (phase 11)

A quick pass over the main screens before the polish (agent area and terminals, API chat,
files and editor, Usage, Agents library, workstreams and the Stage, settings). The biggest
inconsistencies, which this phase fixes:

1. **Many one-off controls.** Buttons, pills and tabs are styled per screen (`af-mode-pill`,
   `af-usage-tabs`, `af-segment`, `af-ws-toggle`, `af-link-btn`, `af-choice`, starter
   cards), with different paddings, radii (3/4/6/8 px) and hover rules; none has a press or
   focus state of its own.
2. **No type scale.** Font sizes drift between 11, 11.5, 12, 12.5, 13, 14 and 15 px; the
   interface uses the system font, so it looks different on every OS.
3. **Pixel UI icons.** Activity bar, panel headers and file tree use 8x8/12x12 pixel icons
   next to smooth text; file types are told apart by color only.
4. **Flat, motionless controls.** Nothing reacts to press, toggles and tabs jump, dialogs
   and toasts pop in, focus rings are 1 px and hard to see.
5. **Hidden sidebar is hard to get back.** Collapsing it leaves no handle; there is no
   shortcut and no matching toggle for the bottom terminal group or the Files panel.
6. **Library Browse forgets filters** (search, filters and scroll) when switching tabs.
7. **Dialogs do not trap focus** (Tab leaves the workstream dialog and the question dialog).
8. **Empty states differ** (plain hint text, a big pixel icon, or nothing); first launch drops
   new users into an empty layout with no guidance.
9. **Characters move in steps**: the prompt bar walk and Stage travel jump between whole
   positions, state changes cut hard, idle is static.
10. **Links** in chat and library items are plain text links without any hint of the site.
