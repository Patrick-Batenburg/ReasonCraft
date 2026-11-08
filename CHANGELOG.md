# Changelog

All notable changes to ReasonCraft are listed here.
A version's heading says "Unreleased" until its release is prepared; then it gets the release date (docs/release.md, release checklist).

## [0.1.0] — Unreleased

The first release.

### Building maps
- Claims, reasons, objections, rebuttals and co-premises, added from the keyboard (Tab, Shift+Tab, Ctrl+Enter) or the ribbon; reasons and objections against a whole card.
- Grouping maps with categories, and notes under any box.
- A parking area for items not connected yet, with **Connect to…** and drag and drop.
- Reordering by dragging or with Alt+arrows; changing a reason into an objection and back.
- Seventeen source types with their own icons, links on boxes, and images in boxes (prepared in the background).
- The Building panel: boxes, sources, comment presets and templates to click or drag onto the map.
- Undo and redo for every change.

### Evaluating
- Accept, doubt or reject claims and premises; rate cards strong, weak or nil, and sources solid, shaky or nil.
- Ratings tint boxes, colour borders, label cards and weight connectors; markers and shading can be turned off.
- Comments on any box, with twelve presets.
- The Check tab: hints from the course rules (missing key words, two statements in one, questions, unsupported premises, partly rated maps, ratings that disagree, parked items), each of which can be ignored or turned off.

### Layout and view
- Automatic compact layout, top-down, bottom-up or left to right.
- Curved, elbow or tapered connectors; arrowheads kept apart where many connectors meet a box.
- Minimap, outline numbers on the map, zoom to fit the map or the selection.
- Light and dark themes, and an interface size from 80 % to 200 %.
- Small windows: ribbon groups shrink from the right (labels first, then folded into drop-downs, as in WordCraft); only when that is not enough does the row scroll, with thin ‹ › arrows over its ends or the mouse wheel. The side panels give way so the map keeps room.

### Writing
- The Text panel: the Outline, the Essay (a text field for each part, ordered strongest first), the Check hints, Snippets of source text and the Help.
- Snippets dragged onto the map become reasons with their source.
- Find and replace in all box text; spellcheck in English and Dutch while typing and with F7, with a personal dictionary.

### Files and output
- `.rcraft` files (a zip with the map, the essay, the snippets and the images); files from newer versions keep the parts this version does not know.
- Autosave to a recovery folder and recovery after a crash.
- `REASONCRAFT_PROFILE_DIR` keeps the settings and the recovery folder in one folder of your choice (portable use, testing).
- One window per user: a map opened while ReasonCraft runs opens in a tab of the running window.
- Export as PNG, SVG or PDF (with page setup and print), copy as image, the outline and the essay as Markdown, plain text or Word, and Argdown export and import.
- Recent files, two example maps and four templates (Blank reasoning map, Blank grouping map, Essay plan, Peer review).

### Languages
- The interface in English and Dutch, following the computer's language until you choose; it switches at once.
- Each map has its own language, which decides the labels on the map, the linking words, the spelling dictionary, the checks and the exports.

### Help and accessibility
- Offline help in English and Dutch (17 topics), with F1 for the topic of the part you are in, and an About page with the licences.
- Every control has a name for screen readers; F6 and Shift+F6 move between the panes; the keyboard reaches every field.
- Search commands (Alt+Q) finds any command by name.
- Keyboard shortcuts for Windows and Linux, and macOS keys of their own (Cmd for Ctrl, Option for Alt).
- **Reset to defaults** in File → Settings, keeping recent files and the personal dictionary.

### Installing
- A Windows MSI for x64, x86 and arm64: for the current user by default (no administrator rights) or for everyone on the computer, with a Start menu shortcut and the `.rcraft` file association; and a portable zip.
- Linux packages for x86_64 and aarch64: `.deb`, `.rpm`, AppImage (with delta updates), Flatpak and a `.tar.gz`, with the `.rcraft` file type.
- A macOS universal app on a `.dmg` (built in CI, not yet checked on a Mac).
- `SHA256SUMS.txt` and build provenance (`gh attestation verify`) with every release, and `reasoncraft --version`.
- The app starts on machines without a usable GPU (software rendering, then OpenGL) and shows an error dialog if no renderer works.

[0.1.0]: https://github.com/Patrick-Batenburg/ReasonCraft/releases/tag/v0.1.0
