# ReasonCraft

ReasonCraft is a free, open-source desktop app for argument mapping. It lets you lay out a claim, the reasons that support it, the objections that oppose it and the rebuttals to those objections as a clear tree of boxes and connectors. Maps are saved as `.rcraft` files, everything works offline, and the app is built natively in Rust with egui for Windows, macOS and Linux.

## What you can do

- **Build the argument.** Add reasons, objections, rebuttals and co-premises by keyboard or from the ribbon, attack a whole card, and group items under categories and notes. Items you have not connected yet wait in the parking area until you drag them into the map or use **Connect to…**.
- **Reorder.** Drag an item between its siblings, or press Alt+Left / Alt+Right.
- **Back claims with sources.** Seventeen source types (expert opinion, statistic, publication, web, quote, personal experience and more), each with its own icon. Add a source from the Home ribbon, the "+" menu, Ctrl+Shift+E or the building panel, and change its type later.
- **Use the building panel.** View → Building panel lists boxes, sources and preset comments. Click an entry to insert it at the selection, or drag it onto the map. Targets that cannot take it show red, and the drop does nothing.
- **Evaluate.** On the Evaluate tab (or with Alt+1/2/3/0), accept, doubt or reject claims and premises, rate cards strong, weak or nil, and rate sources solid, shaky or nil. Ratings tint boxes, colour borders, label cards and set connector weight. Markers and shading can be switched off, and you can clear one rating or all of them.
- **Comment.** Add your own comments or one of twelve presets (positive, needs work, problem) to any box. A badge shows the count, and Ctrl+Shift+M opens them. Set the name shown on your comments in File → Settings.
- **Change types.** Turn a reason into an objection or back again (its strength is kept), and change a source's type.
- **Link.** Attach a web or mail link to a box. A link opens in your browser only when you click its glyph, and only if it is a safe http, https or mailto link.
- **Add images.** Put a PNG, JPEG or WebP image in a box. Images are stored as PNG inside the `.rcraft` file, scaled to at most 1600 px on the longest side, and kept out of undo history.

Files from older versions open unchanged, and files from newer versions keep any parts this version does not understand when you save them.

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
| Ctrl+Shift+R | Add a reason for the whole card (of the selected card or premise) |
| Ctrl+Shift+O | Add an objection to the whole card (of the selected card or premise) |
| Ctrl+Shift+E | Add a source (under the selected claim or premise, or loose) |
| Enter or F2 | Edit the selected box |
| Esc | Stop editing |
| Delete | Delete (items under it move to the parking area) |
| Shift+Delete | Delete with everything under it |
| Arrow keys | Move the selection (Up from a premise selects its card; Down from a card selects its first premise) |
| Ctrl+Shift+P | Jump between the main map and the parking area |
| Ctrl+Shift+M | Open the comments of the selected box (Esc closes them) |
| Ctrl+Shift+K | Connect to…: pick where the selected parked item goes (see below) |
| Alt+Left / Alt+Right | Move the selection one place left / right among its siblings (a premise within its card) |
| Alt+1 | Rate the selection high: accept a claim or premise, strong card, solid source |
| Alt+2 | Rate the selection mid: unsure, weak card, shaky source |
| Alt+3 | Rate the selection low: reject a claim or premise, nil card or source |
| Alt+0 | Clear the selection's rating |
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
| Ctrl+Shift+R | Finish and add a reason for the whole card |
| Ctrl+Shift+O | Finish and add an objection to the whole card |
| Ctrl+Shift+E | Finish and add a source |
| Ctrl+S | Finish and save |
| Ctrl+Shift+S | Finish and save as |
| Ctrl+O | Finish and open |
| Ctrl+N | Finish and start a new map |

All other keys are typed into the box.

### While connecting a parked item (Connect to…)

Connect to… works on an item at the top of the parking area (or a premise of a parked card, which connects its card).

| Keys | Action |
| --- | --- |
| Click a box or card | Connect the parked item there (targets that refuse it show red and do nothing) |
| Arrow keys | Move the selection in the map |
| Enter or F2 | Connect the parked item to the selected box or card |
| Ctrl+Shift+P, zoom keys | Work as usual |
| Esc | Cancel |

Other keys do nothing until you connect or cancel.

Dragging also reorders: drop an item between its siblings (an insertion bar shows where), or a premise between the premises of its card.

## Licence

Licensed under either of MIT or Apache-2.0, at your option.
