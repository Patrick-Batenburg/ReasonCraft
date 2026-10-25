# ReasonCraft

ReasonCraft is a free, open-source desktop app for argument mapping. It lets you lay out a claim, the reasons that support it, the objections that oppose it and the rebuttals to those objections as a clear tree of boxes and connectors. Maps are saved as `.rcraft` files, everything works offline, and the app is built natively in Rust with egui for Windows, macOS and Linux.

## Build and run

You need [rustup](https://rustup.rs). The Rust version is pinned in `rust-toolchain.toml`, so rustup installs and uses the right toolchain by itself the first time you build.

```sh
cargo run -p reasoncraft --release
```

To open maps straight away, pass their paths: `cargo run -p reasoncraft --release -- my-map.rcraft`.

Run the checks with:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

A debug build (`cargo run -p reasoncraft`) adds **View → Load sample map (debug)**, which opens a generated map of 117 boxes.

## Keyboard shortcuts

Ctrl means Cmd on macOS. Typing while a box is selected starts editing that box.

### While a box or card is selected

| Keys | Action |
| --- | --- |
| Tab | Add a reason |
| Shift+Tab | Add an objection |
| Ctrl+Enter | Add a co-premise |
| Enter or F2 | Edit the selected box |
| Esc | Stop editing |
| Delete | Delete (items under it move to the parking area) |
| Shift+Delete | Delete with everything under it |
| Arrow keys | Move the selection |
| Ctrl+Z | Undo |
| Ctrl+Y or Ctrl+Shift+Z | Redo |
| Ctrl+S | Save |
| Ctrl+Shift+S | Save as |
| Ctrl+O | Open |
| Ctrl+N | New map |
| Ctrl+0 | Zoom to 100% |
| Ctrl+1 | Zoom to fit |
| Ctrl+= or Ctrl++ | Zoom in |
| Ctrl+- | Zoom out |

### While editing a box's text

| Keys | Action |
| --- | --- |
| Esc | Finish editing |
| Tab | Finish and add a reason |
| Shift+Tab | Finish and add an objection |
| Ctrl+Enter | Finish and add a co-premise |
| Ctrl+S | Finish and save |
| Ctrl+Shift+S | Finish and save as |
| Ctrl+O | Finish and open |
| Ctrl+N | Finish and start a new map |

All other keys are typed into the box.

## Licence

Licensed under either of MIT or Apache-2.0, at your option.
