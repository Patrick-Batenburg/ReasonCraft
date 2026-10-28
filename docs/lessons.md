# Lessons

Patterns learned from corrections during this project. Review at session start.

## Mockups and rendering
- **Fix the drawing size of SVG mockups.** An SVG with `preserveAspectRatio="none"` stretched to a wider container detaches its lines from absolutely positioned boxes (the "red bar" bug). Give the canvas a fixed width or compute coordinates from measured element positions.
- **Measure text, never estimate it.** Character-count height estimates clipped long box text. Measure the real wrapped text (DOM `offsetHeight` in mockups, egui galley in the app).
- **Render and look before showing.** Render mockups headlessly (Edge `--headless --screenshot`) and inspect crops of the busy areas before handing them over.

## Argument-map semantics
- **Distinguish whole-card targets from single-premise targets.** An argument against a whole card connects to the card frame's edge, not to a premise or through the boxes.
- **Keep labels out of connector paths.** Put rating labels inside the card frame, align arrowheads with a straight stub, and spread connectors that share an attach point.

## Agent safety
- **Never inject OS-level keyboard/mouse input (SendKeys, SendInput, xdotool) into the desktop for manual checks.** Focus can change to another application and keystrokes land there. Drive the app through kittests, debug-only scenario loaders, or `--scenario` style startup flags; screenshots of the app window are fine.

## Build machine
- **Always run workspace builds and tests with `-j 4`.** Linking ~30 app test binaries in parallel with MSVC `link.exe` (~2.7 GB each) once exhausted the machine's memory.
- **Check a release build too, not only the debug tests.** Debug-only items (`cfg(debug_assertions)`) referenced from normal code compile in tests but break `--release`; the CI runs `cargo check --workspace --release`.
- **Check the machine before chasing "impossible" memory corruption.** Random crashes in safe code under heavy load on this machine came from its processor (WHEA-Logger event 19, corrected machine checks; the processor on an old BIOS). Look at `Get-WinEvent -FilterHashtable @{LogName='System'; ProviderName='Microsoft-Windows-WHEA-Logger'}` first.
- **Git Bash turns off Windows Error Reporting for what it starts**, so a crash there leaves no dump and no event (exit 139 is an access violation, 127 an abort). Start crash hunts from PowerShell, or catch the first fatal exception with a vectored handler that writes a minidump.
