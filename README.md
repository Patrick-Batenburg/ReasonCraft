# ReasonCraft

ReasonCraft is a free, open-source desktop app for argument mapping. It lets you lay out a claim, the reasons that support it, the objections that oppose it and the rebuttals to those objections as a clear tree of boxes and connectors. Maps are saved as `.rcraft` files, everything works offline, and the app is built natively in Rust with egui for Windows, macOS and Linux.

## What you can do

- **Build the argument.** Add reasons, objections, rebuttals and co-premises by keyboard or from the ribbon, attack a whole card, and group items under categories and notes. Items you have not connected yet wait in the parking area until you drag them into the map or use **Connect to…**.
- **Reorder.** Drag an item between its siblings, or press Alt+Left / Alt+Right.
- **Back claims with sources.** Seventeen source types (expert opinion, statistic, publication, web, quote, personal experience and more), each with its own icon. Add a source from the Home ribbon, the "+" menu, Ctrl+Shift+E or the building panel, and change its type later.
- **Use the building panel.** View → Building panel lists boxes, sources and preset comments. Click an entry to insert it at the selection, or drag it onto the map. Targets that cannot take it show red, and the drop does nothing.
- **Evaluate.** On the Evaluate tab (or with Alt+1/2/3/0, and Alt+Shift+0 to clear the whole map), accept, doubt or reject claims and premises, rate cards strong, weak or nil, and rate sources solid, shaky or nil. Ratings tint boxes, colour borders, label cards and set connector weight. Markers and shading can be switched off, and you can clear one rating or all of them.
- **Comment.** Add your own comments or one of twelve presets (positive, needs work, problem) to any box. A badge shows the count, and Ctrl+Shift+M opens them. Set the name shown on your comments in File → Settings.
- **Write from the Text panel.** View → Text panel opens the panel on the right, with four tabs (see [The Text panel](#the-text-panel)): an **Outline** of the map, an **Essay** tab that arrives in a later version, a **Check** tab with course-rule hints, and **Snippets** of source text to build the map from.
- **Set the document language.** File → Document holds the title, author and language (English or Dutch) of the open map. The language drives the "and"/"or" and key-term checks; File → Settings sets the language new documents start with.
- **Change types.** Turn a reason into an objection or back again (its strength is kept), and change a source's type.
- **Link.** Attach a web or mail link to a box. A link opens in your browser only when you click its glyph, and only if it is a safe http, https or mailto link.
- **Add images.** Put a PNG, JPEG or WebP image in a box. Images are stored as PNG inside the `.rcraft` file, scaled to at most 1600 px on the longest side, and kept out of undo history.

Files from older versions open unchanged, and files from newer versions keep any parts this version does not understand when you save them.

## The Text panel

- **Outline.** The map as a numbered list in canvas order: `1`, `1.1`, … for cards, letters for co-premises (`1a`, `1b`), `.w1` for an attack on a whole card, and `P1`, `P2` for items in the parking area. Sources sit unnumbered under their box. Switches turn the numbers, the linking words (because, but, however), indenting, role colours and notes on or off; your choice is remembered. Click a line to select its box on the map. **Copy all** puts the outline on the clipboard as indented plain text.
- **Check.** Hints based on the course rules: missing key words (a word of the statement above that none of a card's premises uses), two statements in one ("and"/"or", or "en"/"of" in Dutch), questions, premises nothing supports, empty top-level categories, a partly rated map, ratings that disagree, and items in the parking area. Click a hint to select the item and bring it into view. The status bar shows the number of hints; click it to open this tab. Each check can be turned off in File → Settings. Checks are hints only: they never block anything.
- **Snippets.** Keep pieces of source text beside the map. **New snippet** adds an empty one; **Paste as snippet** (or Ctrl+V while the pointer is over the tab and no text field has focus) adds the clipboard text. A snippet holds up to 50,000 characters; longer text is cut, with a notice. Tag a snippet with a source type, a citation and a link, and drag the handle to reorder the list.
- **Build the map from snippets.** Select text in a snippet and drag the **Drag to map** chip onto the map, or hold Alt and drag a snippet's handle to take its whole text. On a claim or premise the text becomes a new reason under it, on a card a reason for the whole card, on empty canvas a reason in the parking area, and on a category a new category. The snippet's source goes under the new premise, and the drop is one undo step. Change the new card's type afterwards with **Type**. Blank text, notes and sources refuse the drop and show red.

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
| Alt+Shift+0 | Clear every rating in the map |
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
