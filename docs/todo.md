# ReasonCraft — Status

**Active:** none. The pre-release review (2026-11, seven seats, all approved) is merged; its decisions are in [rulings.md](rulings.md) ("Made in the pre-release review").
**Done:** steps 1–6, packaging and the full code review are merged into `main`. Their plans, the long status log and the lessons are in [docs/dev/](dev/README.md): [todo-history.md](dev/todo-history.md), [lessons.md](dev/lessons.md) (review it at session start).

## Open
- [ ] Before the first release: create the repo as exactly `Patrick-Batenburg/ReasonCraft`, follow "First-time GitHub setup" in [release.md](release.md) (incl. private vulnerability reporting), then the dry run
- [ ] A file handed to the running window while an error, export or link dialog is open waits for that dialog; if the window is then closed with no unsaved work before the dialog is dismissed, that file is not opened (ruling RR-2)
- [ ] Once a remote exists: the first release-workflow run (macOS universal DMG, signing and notarization when the secrets exist, Flatpak on both architectures, Windows arm64, Linux aarch64) and the packaging lint run; CI green on Windows, macOS and Linux (macOS code has never compiled or run)
- [ ] Release checklist in [release.md](release.md): interactive installer run in the VM (licence page), 200 % UI-scale check, a real Mac check of the DMG, Wayland and the real Linux clipboard, Word/LibreOffice open checks; an install smoke test of the arm64 MSI on `windows-11-arm`
- [ ] macOS: open a double-clicked `.rcraft` (Apple Event, ruling S6-T11-2), custom app menu (Hide/Quit through the unsaved prompt), Dock-quit prompt; a DMG background (needs a design)
- [ ] Consider collecting Apache NOTICE files (ruling PK-1); drop the Noto Emoji OFL entry if the font isn't compiled in
- [ ] Owner review of the Dutch glossary ([glossary-nl.md](glossary-nl.md))
- [ ] Split files at ~400 lines before the next feature (canvas/input.rs, shell/icons/mod.rs)
- [ ] Open still runs on the UI thread (ruling B6-5); save is a background job
- [ ] Screen-reader tree only covers items on screen (B7-8)
- [ ] Deleting a very large branch is quadratic (edit/move_remove.rs `shift_remove`), a one-off cost
- [ ] Layout reruns every frame (linear in map size); caching per revision would cut idle-redraw cost on huge maps
- [ ] Smaller: Recent shows `\?\UNC` paths for network shares; profile warning invisible in GUI builds; per-frame Fluent lookups in the scene; thousands separators
- [ ] If a test-run abort ever appears on CI hardware, ruling R2-6 no longer holds
