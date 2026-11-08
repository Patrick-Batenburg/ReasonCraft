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
- **Check the machine before chasing "impossible" memory corruption.** Random crashes in safe code under heavy load can be hardware faults. Look at the system's hardware error log first.
- **Git Bash turns off Windows Error Reporting for what it starts**, so a crash there leaves no dump and no event (exit 139 is an access violation, 127 an abort). Start crash hunts from PowerShell, or catch the first fatal exception with a vectored handler that writes a minidump.

## A test that starts the real exe must isolate its profile from the first run
A `--version` test, while still RED, launched the real app window with the owner's real profile and left a recovery session folder behind. Any test (or script) that spawns `reasoncraft.exe` sets `REASONCRAFT_PROFILE_DIR` to a temp dir and kills the process on timeout — even when it only expects a quick exit, because the RED run is the one that won't exit.

## CI on other platforms
- **Compare paths in tests the way the app stores them.** macOS's temporary folder is behind the `/var` → `/private/var` link, so a test comparing a stored, resolved path with the path it saved under passes on Windows and Linux and fails on macOS. Resolve both sides (`canonicalize`).
- **A test of drawn UI must follow the platform's branches.** Where the code draws something only on some systems (`cfg!(target_os = ...)`), the test's expectation takes the same branch.
- **In PowerShell, build a splatted argument list with `@(...)` around the whole expression.** `$x = if (...) { @('-g') }` unwraps to the string `'-g'`, and `@x` then passes it per character. Run each code path of a packaging script once (here the global extension install was only ever run on CI).
- **The first push into a new repository may start no workflow.** Start CI by hand after creating the repo (`gh workflow run`).

## Agent tooling
- **Never write backslashes through a Bash heredoc into a script that writes files.** The shell tool turned the two-character sequences backslash-v and backslash-t inside a quoted heredoc into a vertical tab and a tab, which corrupted a path in the docs and a PowerShell line. Use the Edit/Write tools for text with backslashes, and grep the changed files for control characters (`grep -P '[\x00-\x08\x0b\x0c\x0e-\x1f]'`) before committing.
