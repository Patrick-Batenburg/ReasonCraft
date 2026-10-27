# ReasonCraft

ReasonCraft is a free, open-source desktop app for argument mapping. It lets you lay out a claim, the reasons that support it, the objections that oppose it and the rebuttals to those objections as a clear tree of boxes and connectors. Maps are saved as `.rcraft` files, everything works offline, and the app is built natively in Rust with egui for Windows, macOS and Linux.

## What you can do

- **Build the argument.** Add reasons, objections, rebuttals and co-premises by keyboard or from the ribbon, attack a whole card, and group items under categories and notes. Items you have not connected yet wait in the parking area until you drag them into the map or use **Connect to…**.
- **Reorder.** Drag an item between its siblings, or press Alt+Left / Alt+Right (Alt+Up / Alt+Down in a left-to-right map).
- **Back claims with sources.** Seventeen source types (expert opinion, statistic, publication, web, quote, personal experience and more), each with its own icon. Add a source from the Home ribbon, the "+" menu, Ctrl+Shift+E or the building panel, and change its type later.
- **Use the building panel.** View → Building panel lists boxes, sources and preset comments. Click an entry to insert it at the selection, or drag it onto the map. Targets that cannot take it show red, and the drop does nothing.
- **Evaluate.** On the Evaluate tab (or with Alt+1/2/3/0, and Alt+Shift+0 to clear the whole map), accept, doubt or reject claims and premises, rate cards strong, weak or nil, and rate sources solid, shaky or nil. Ratings tint boxes, colour borders, label cards and set connector weight. Markers and shading can be switched off, and you can clear one rating or all of them.
- **Comment.** Add your own comments or one of twelve presets (positive, needs work, problem) to any box. A badge shows the count, and Ctrl+Shift+M opens them. Set the name shown on your comments in File → Settings.
- **Write from the Text panel.** View → Text panel opens the panel on the right, with four tabs (see [The Text panel](#the-text-panel)): an **Outline** of the map, an **Essay** written along the map (see [The Essay tab](#the-essay-tab)), a **Check** tab with course-rule hints, and **Snippets** of source text to build the map from.
- **Set the document language.** File → Document holds the title, author and language (English or Dutch) of the open map. The language drives the "and"/"or" and key-term checks; File → Settings sets the language new documents start with.
- **Lay the map out your way.** View → Layout direction draws the map top-down (the default), bottom-up or left to right; the direction is saved with the map, and changing it is one undo step. In a left-to-right map the co-premises of a card stack vertically. View → Connectors draws the lines curved, as elbows or as tapered bands that widen toward the parent; this is your own preference, kept for every map. Exports use the direction and style you see.
- **Keep your bearings.** View → Minimap shows an overview of the whole map in the corner of the canvas, with the part in view outlined; click or drag in it to move there. By default it appears only when the map does not fit the window, and it can also be always on or off. View → Numbers puts the Outline's numbers (1, 1a, 1.w1, P1) on the boxes, on screen and in exports.
- **Search commands.** The box in the title bar (Alt+Q) finds any command by name, including the ones without a ribbon button, and runs it with Enter.
- **Find and replace.** Ctrl+F (or Find… in the Home ribbon) opens a find bar above the map that searches the text of every box: claims, premises, categories, sources and notes, the main map first, then the parking area. Match case and whole word are switches in the bar; the counter shows which hit you are on, the hit's box is selected and brought into view, and every box with a hit gets an outline. Ctrl+H (Replace…) adds Replace and Replace all; Replace all is one undo step.
- **Check spelling.** Misspelled words get a red underline while you type in a box, an essay paragraph, a snippet or a comment, using the English or Dutch dictionary of the document's language (bundled, so it works offline). F7 (or Spelling in the Home ribbon) walks the misspellings of box text, sources and notes from the selection, with suggestions and Change, Change all, Ignore, Ignore all and Add to dictionary; Change and Change all are one undo step each. Underlines never show on the map or in exports. Turn spellcheck off in File → Settings.
- **Autosave and recovery.** While you work, ReasonCraft keeps a copy of every map with unsaved changes in a recovery folder, every 45 seconds by default (File → Settings: 30 to 600 seconds, or 0 to turn it off). Your own files are only written when you save. If the app does not close normally, the next start offers to restore those maps (Restore all, Discard all or Decide later); a restored map opens as "(recovered)" with its unsaved changes, and if its file changed in the meantime you are asked before it is replaced. The recovery folder is:
  - Windows: `%LOCALAPPDATA%\ReasonCraft\recovery`
  - macOS: `~/Library/Application Support/ReasonCraft/recovery`
  - Linux and other Unix: `$XDG_DATA_HOME/ReasonCraft/recovery`, or `~/.local/share/ReasonCraft/recovery` when that is not set
- **Accessibility and comfort.** Every control has a name for screen readers. On the map Tab adds a reason; F6 and Shift+F6 move between the ribbon, the panels, the map and the status bar, and once a control has the focus Tab and Shift+Tab move through the controls, the find bar and the panels, including the long Essay and Check lists, which scroll along. File → Settings sets the interface size (80 % to 200 %, on top of the map's own zoom), and **Reset to defaults** puts every setting back as it was at the first start, keeping your personal dictionary.
- **Change types.** Turn a reason into an objection or back again (its strength is kept), and change a source's type.
- **Link.** Attach a web or mail link to a box. A link opens in your browser only when you click its glyph, and only if it is a safe http, https or mailto link.
- **Add images.** Put a PNG, JPEG or WebP image in a box. The image is prepared in the background, so a large photo never freezes the window: the box shows "Preparing image…" until it is ready. Images are stored as PNG inside the `.rcraft` file, scaled to at most 1600 px on the longest side, and kept out of undo history.
- **Export and print.** The Export tab saves the map as PNG, SVG or PDF, copies it to the clipboard as an image, prints it, saves the outline or the essay as Markdown, plain text or Word, and saves the whole map as Argdown text (see [Export and print](#export-and-print)).
- **Import Argdown.** File → Import Argdown… reads an [Argdown](https://argdown.org) file into a new, unsaved tab and lays it out. Argdown the map cannot hold (contradictions, tags, inference rules, a statement used in two places, and so on) is changed or left out, and a report lists each case with its line. The file itself is never written; use Save As to keep the map. See [docs/argdown.md](docs/argdown.md#reading-argdown).

Files from older versions open unchanged, and files from newer versions keep any parts this version does not understand when you save them.

## The Text panel

- **Outline.** The map as a numbered list in canvas order: `1`, `1.1`, … for cards, letters for co-premises (`1a`, `1b`), `.w1` for an attack on a whole card, and `P1`, `P2` for items in the parking area. Sources sit unnumbered under their box. Switches turn the numbers, the linking words (because, but, however), indenting, role colours and notes on or off; your choice is remembered. Click a line to select its box on the map. **Copy all** puts the outline on the clipboard as indented plain text.
- **Check.** Hints based on the course rules: missing key words (a word of the statement above that none of a card's premises uses), two statements in one ("and"/"or", or "en"/"of" in Dutch), questions, premises nothing supports, empty top-level categories, a partly rated map (one hint with the number of unrated items; click it to go to the first), ratings that disagree, and items in the parking area (notes there are left alone, and parked items are drafts: they are not asked for support or a rating). Click a hint to select the item and bring it into view. The status bar shows the number of hints; click it to open this tab. Each check can be turned off in File → Settings. Checks are hints only: they never block anything.
- **Snippets.** Keep pieces of source text beside the map. **New snippet** adds an empty one; **Paste as snippet** (or Ctrl+V while the pointer is over the tab and no text field has focus) adds the clipboard text. A snippet holds up to 50,000 characters; longer text is cut, with a notice. Tag a snippet with a source type, a citation and a link, and drag the handle to reorder the list.
- **Build the map from snippets.** Select text in a snippet and drag the **Drag to map** chip onto the map, or hold Alt and drag a snippet's handle to take its whole text. On a claim or premise the text becomes a new reason under it, on a card a reason for the whole card, on empty canvas a reason in the parking area, and on a category a new category. The snippet's source goes under the new premise, and the drop is one undo step. Change the new card's type afterwards with **Type**. Blank text, notes and sources refuse the drop and show red.

## The Essay tab

The Essay tab in the Text panel turns the map into the outline of an essay and gives you a text field for each part:

- **Introduction** shows the claim; write the paragraph that introduces it.
- **One section per branch** under the claim, strongest first: strong cards, then unrated ones, then weak, then nil, and within the same strength reasons before objections. A section lists its premises with their evidence, then the objections and the rebuttals that answer them, each with a field for its paragraph. The dots in a section's heading show the card's strength.
- **Conclusion** shows the claim again; sum up the main reasons.
- Click a statement to select it on the map and bring it into view.
- Typing in a field is one undo step, from the first key to leaving the field.
- **Not in the essay right now** lists text written for items that are parked or no longer part of an argument. It is kept, and comes back into its section when the item returns.
- **Unplaced notes:** when you delete an item that has essay text, the text moves here with the item's text as its label, so nothing you wrote is lost. **Copy text** puts it on the clipboard and **Remove note** deletes it (undo brings it back, and undoing the delete puts the text back in its place).

The essay is saved inside the `.rcraft` file. Earlier versions of ReasonCraft open such files without showing the essay, and keep it when they save.

## Export and print

Everything is on the **Export** tab of the ribbon. Exports run in the background: the status bar shows "Exporting…", then "Export done." with an **Open folder** link. Status messages go away after a few seconds, or when you next change the map or give a file or export command.

- **PNG, SVG, PDF.** A dialog first asks what to export (the whole map or the selection and what is under it) and in which colours (the light theme, the default, or the theme you are using).
  - *PNG:* scale 1× to 4×, and an optional transparent background. Images are limited to 16,384 px on a side and 256 megapixels; for larger maps, use SVG or PDF.
  - *SVG:* text is turned into outlines by default, so the drawing looks the same on a computer without the font. Emoji are left out of outlined text. Untick the option to keep live, selectable text.
  - *PDF:* text stays selectable, and the font is embedded. **Page setup…** sets the paper (A4 or Letter), orientation, margin and fit: the whole map on one page (small maps are not enlarged beyond normal size), or across pages at a chosen scale, with each page labelled "row/column".
- **Copy as image** puts a 2× PNG of the map, in the colours you see, on the clipboard (up to 64 megapixels).
- **Print** makes a PDF with the page setup and opens it in your PDF viewer, where you print it.
- **Outline** (menu): the outline as Markdown, plain text or a Word document, with the numbering and switches of the Outline tab.
- **Essay** (menu): the essay as Markdown or a Word document, with the document's title and author and a heading for each part.
- **Argdown** saves the whole map, with the parking area, as [Argdown](https://argdown.org) text that other Argdown tools can read. Images are left out. The mapping is described in [docs/argdown.md](docs/argdown.md).

Exports show exactly what the map shows (layout, wrapped lines, colours, ratings, source icons and images) without the screen-only parts: the selection, the "+" button, comment badges, link glyphs, find highlights, the minimap and the dot grid. In the outline and essay, the linking words ("because"/"omdat", "but"/"maar") and source names follow the document language (English or Dutch). Word files use the built-in Title and Heading styles, so they open in Word and LibreOffice and pick up their styles.

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

macOS has its own column: there Cmd takes the place of Ctrl and Option of Alt, and Control is a key of its own. On a Mac keyboard Backspace is the key labelled delete, Delete is Fn+delete, and the F-keys (F2, F3, F6, F7) need Fn unless they are set to work as standard function keys. Typing while a box is selected starts editing that box. Keys marked (context) only work where the table says: elsewhere, for example in a text field, they type or move the focus as usual.

| Windows / Linux | macOS | Action |
| --- | --- | --- |
| Alt+Q | Shift+Cmd+A | Search commands, also while typing in a box or field: type part of a command's name, pick it with the arrow keys and press Enter (Esc closes the list) |
| Ctrl+F | Cmd+F | Find in box text, sources and notes, also while typing in a box or field: the find bar opens above the map (in its field Enter / Shift+Enter go to the next / previous hit, Esc closes it) |
| Ctrl+H | Option+Cmd+F | Find and replace |
| F3 / Shift+F3 | F3 / Shift+F3 | Go to the next / previous hit |
| F7 | F7 | Spelling, also while typing in a box or field: check the spelling of box text, sources and notes from the selection, one misspelling at a time |
| F6 / Shift+F6 | F6 / Shift+F6 | Move the keyboard focus to the next / previous pane, from any control: ribbon, Building panel, map, Text panel, status bar (hidden panels are skipped) |

### While a box or card is selected

| Windows / Linux | macOS | Action |
| --- | --- | --- |
| Tab | Tab | Add a reason (context) |
| Shift+Tab | Shift+Tab | Add an objection (context) |
| Ctrl+Enter | Cmd+Enter | Add a co-premise |
| Ctrl+Shift+R | Shift+Cmd+R | Add a reason for the whole card (of the selected card or premise) |
| Ctrl+Shift+O | Shift+Cmd+O | Add an objection to the whole card (of the selected card or premise) |
| Ctrl+Shift+E | Shift+Cmd+E | Add a source (under the selected claim or premise, or loose) |
| F2 | F2 | Edit the selected box |
| Enter | Enter | Edit the selected box (context) |
| Esc | Esc | Stop editing (context) |
| Delete | Delete | Delete (items under it move to the parking area) |
| – | Backspace | Delete, like Delete (context) |
| Shift+Delete | Shift+Delete | Delete with everything under it |
| – | Shift+Backspace | Delete with everything under it, like Shift+Delete (context) |
| Arrow keys | Arrow keys | Move the selection (Up from a premise selects its card; Down from a card selects its first premise). The arrows follow the layout direction: bottom-up, Down goes to the parent; left-to-right, Left goes to the parent and Up/Down move between siblings (context) |
| Ctrl+Shift+P | Shift+Cmd+P | Jump between the main map and the parking area |
| Ctrl+Shift+M | Shift+Cmd+M | Open the comments of the selected box (Esc closes them) |
| Ctrl+Shift+K | Shift+Cmd+K | Connect to…: pick where the selected parked item goes (see below) |
| Alt+Left / Alt+Right / Alt+Up / Alt+Down | Option+Left / Option+Right / Option+Up / Option+Down | Move the selection one place among its siblings (a premise within its card): Left / Right in top-down and bottom-up maps, Up / Down in left-to-right maps, where siblings stack vertically |
| Alt+1 | Option+Cmd+1 | Rate the selection high: accept a claim or premise, strong card, solid source |
| Alt+2 | Option+Cmd+2 | Rate the selection mid: unsure, weak card, shaky source |
| Alt+3 | Option+Cmd+3 | Rate the selection low: reject a claim or premise, nil card or source |
| Alt+0 | Option+Cmd+0 | Clear the selection's rating |
| Alt+Shift+0 | Option+Shift+Cmd+0 | Clear every rating in the map |
| Ctrl+Z | Cmd+Z | Undo |
| Ctrl+Y or Ctrl+Shift+Z | Cmd+Y or Shift+Cmd+Z | Redo |
| Ctrl+S | Cmd+S | Save |
| Ctrl+Shift+S | Shift+Cmd+S | Save as |
| Ctrl+O | Cmd+O | Open |
| Ctrl+N | Cmd+N | New map |
| Ctrl+0 | Cmd+0 | Zoom to 100% |
| Ctrl+1 | Cmd+1 | Zoom to fit |
| Ctrl+Shift+1 | Shift+Cmd+1 | Zoom to fit the selection (a card with everything under it) |
| Ctrl+= or Ctrl++ | Cmd+= or Cmd++ | Zoom in |
| Ctrl+- | Cmd+- | Zoom out |

### While editing a box's text

| Windows / Linux | macOS | Action |
| --- | --- | --- |
| Esc | Esc | Finish editing (context) |
| Tab | Tab | Finish and add a reason (context) |
| Shift+Tab | Shift+Tab | Finish and add an objection (context) |
| Ctrl+Enter | Cmd+Enter | Finish and add a co-premise |
| Ctrl+Shift+R | Shift+Cmd+R | Finish and add a reason for the whole card |
| Ctrl+Shift+O | Shift+Cmd+O | Finish and add an objection to the whole card |
| Ctrl+Shift+E | Shift+Cmd+E | Finish and add a source |
| Ctrl+S | Cmd+S | Finish and save |
| Ctrl+Shift+S | Shift+Cmd+S | Finish and save as |
| Ctrl+O | Cmd+O | Finish and open |
| Ctrl+N | Cmd+N | Finish and start a new map |

All other keys are typed into the box.

### While typing in a snippet, citation, link, title, author or essay field

| Windows / Linux | macOS | Action |
| --- | --- | --- |
| Ctrl+S | Cmd+S | Finish and save |
| Ctrl+Shift+S | Shift+Cmd+S | Finish and save as |
| Ctrl+O | Cmd+O | Finish and open |
| Ctrl+N | Cmd+N | Finish and start a new map |

All other keys stay with the field.

### While connecting a parked item (Connect to…)

Connect to… works on an item at the top of the parking area (or a premise of a parked card, which connects its card).

| Windows / Linux | macOS | Action |
| --- | --- | --- |
| Click a box or card | Click a box or card | Connect the parked item there (targets that refuse it show red and do nothing) (context) |
| Arrow keys | Arrow keys | Move the selection in the map (context) |
| Enter or F2 | Enter or F2 | Connect the parked item to the selected box or card (context) |
| Ctrl+Shift+P, zoom keys | Shift+Cmd+P, zoom keys | Work as usual (context) |
| Esc | Esc | Cancel (context) |

Other keys do nothing until you connect or cancel.

Dragging also reorders: drop an item between its siblings (an insertion bar shows where), or a premise between the premises of its card.

## Known limitations

- A box with many connectors attached grows so that their arrowheads stay apart: wider top-down and bottom-up, taller left-to-right. A box therefore can't be narrower than its connectors need, even when its width is set in the file.

- macOS: the app has no app menu (About, Hide, Quit), so that Cmd+Q reaches the window and asks about unsaved changes first. Quitting from the Dock, or logging out or shutting down, still quits without asking; with autosave on, the next start offers to recover the work up to the last autosave.

## Licence

Licensed under either of MIT or Apache-2.0, at your option.
