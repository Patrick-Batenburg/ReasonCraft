# ReasonCraft UI strings (English).
# Code uses dotted keys; each dot becomes a dash here (ribbon.tab.home -> ribbon-tab-home).

app-name = ReasonCraft
untitled = Untitled
window-title = { $doc } — { $app }

## Title bar
titlebar-search = Search commands
window-minimize = Minimize
window-maximize = Maximize
window-restore = Restore
window-close = Close window

## Document tabs
tabs-new = New tab
tabs-close = Close { $name }

## Ribbon tabs
ribbon-tab-file = File
ribbon-tab-home = Home
ribbon-tab-insert = Insert
ribbon-tab-evaluate = Evaluate
ribbon-tab-view = View
ribbon-tab-export = Export
ribbon-tab-help = Help
ribbon-coming_soon = Coming soon

## Ribbon groups
group-clipboard = Clipboard
group-add = Add
group-editing = Editing
group-zoom = Zoom
group-theme = Theme
group-panels = Panels
group-debug = Debug

## Commands
action-new = New
action-open = Open
action-save = Save
action-save_as = Save As
action-undo = Undo
action-redo = Redo
action-paste = Paste
action-cut = Cut
action-copy = Copy
action-add_reason = Reason
action-add_objection = Objection
action-add_co_premise = Co-premise
action-add_category = Category
action-add_note = Note
action-delete = Delete
action-delete_branch = Delete branch
action-zoom_fit = Fit
action-zoom_100 = 100%
action-zoom_in = Zoom in
action-zoom_out = Zoom out
action-panel_left = Building panel
action-panel_right = Text panel
action-load_sample = Load sample map (debug)

## Files
file-filter = ReasonCraft map
modal-ok = OK
unsaved-title = Save changes to { $name }?
unsaved-message = Your changes will be lost if you don't save them.
unsaved-save = Save
unsaved-discard = Don't save
unsaved-cancel = Cancel
error-open-title = Could not open { $file }
error-save-title = Could not save { $file }
error-open-not_a_file = This is not a ReasonCraft map.
error-open-missing_document = This ReasonCraft file has no map inside.
error-open-malformed = This ReasonCraft file is damaged and cannot be read. ({ $detail })
error-open-newer_version = This map was made with a newer version of ReasonCraft (file version { $found }; this version reads up to version { $supported }). Update ReasonCraft to open it.
error-open-io = The file could not be read: { $detail }
error-save-io = The file could not be written: { $detail }. Your changes are still open and unsaved.
error-save-other = The map could not be saved: { $detail }. Your changes are still open and unsaved.

## Backstage
backstage-back = Back

## Themes
theme-system = System
theme-light = Light
theme-dark = Dark

## Side panels
panel-building = Building
panel-text = Text
panel-coming_later = Coming in a later version.

## Canvas
canvas-empty_claim = Type your main claim
canvas-label = Argument map
editor-hint = Type here
plus-open = Add item
plus-whole_card_reason = Reason for the whole card
plus-whole_card_objection = Objection to the whole card
parking-title = Not connected

## Card roles
role-supports = supports
role-opposes = opposes
role-rebuts = rebuts

## Source types
source-publication = Publication
source-website = Website
source-book = Book
source-quote = Quote
source-expert = Expert
source-data = Data

## Status bar
status-boxes = { $count ->
    [one] { $count } box
   *[other] { $count } boxes
}
status-toggle_theme = Toggle theme
status-zoom = Zoom
status-zoom_value = { $percent }%
