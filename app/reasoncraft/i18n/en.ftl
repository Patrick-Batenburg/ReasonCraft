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
group-eval_claim = Claims and premises
group-eval_card = Cards
group-eval_source = Sources
group-eval_clear = Clear
group-eval_view = Display
group-evaluation = Evaluation

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
action-add_source = Source
action-delete = Delete
action-delete_branch = Delete branch
action-zoom_fit = Fit
action-zoom_100 = 100%
action-zoom_in = Zoom in
action-zoom_out = Zoom out
action-panel_left = Building panel
action-panel_right = Text panel
action-load_sample = Load sample map (debug)
action-source_icons = Source icons (debug)

## Evaluation
eval-accept = Accept
eval-unsure = Unsure
eval-reject = Reject
eval-strong = Strong
eval-weak = Weak
eval-nil = Nil
eval-solid = Solid
eval-shaky = Shaky
eval-clear_selection = Selection
eval-clear_map = Map
eval-show_markers = Show markers
eval-shading = Evaluation shading
eval-tip-claim = Accept, doubt or reject the selected claim or premise (Alt+1, Alt+2, Alt+3; Alt+0 clears)
eval-tip-card = Rate how strongly the selected card's premises carry its conclusion (Alt+1, Alt+2, Alt+3; Alt+0 clears)
eval-tip-source = Rate how reliable the selected source is (Alt+1, Alt+2, Alt+3; Alt+0 clears)
eval-tip-clear_selection = Clear the rating of the selected item (Alt+0)
eval-tip-clear_map = Clear every rating in the map

## Files
file-filter = ReasonCraft map
modal-ok = OK
unsaved-title = Save changes to { $name }?
unsaved-message = Your changes will be lost if you don't save them.
unsaved-save = Save
unsaved-save_changes = Save changes
unsaved-discard = Don't save
unsaved-cancel = Cancel
replace-title = { $file } already exists. Replace it?
replace-message = Its current contents will be overwritten.
replace-confirm = Replace
replace-cancel = Cancel
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
plus-source = Source
parking-title = Not connected

## Card roles
role-supports = supports
role-opposes = opposes
role-rebuts = rebuts

## Card rating labels
rating-strong-supports = ●● strongly supports
rating-strong-opposes = ●● strongly opposes
rating-strong-rebuts = ●● strongly rebuts
rating-weak-supports = ● weakly supports
rating-weak-opposes = ● weakly opposes
rating-weak-rebuts = ● weakly rebuts
rating-nil-supports = does not support
rating-nil-opposes = does not oppose
rating-nil-rebuts = does not rebut

## Source types
source-source = Source
source-my_basis = My own basis
source-assertion = Assertion
source-by_definition = By definition
source-case_study = Case study
source-common_belief = Common belief
source-data = Data
source-event = Event
source-example = Example
source-expert_opinion = Expert opinion
source-law = Law
source-media = Media
source-personal_experience = Personal experience
source-publication = Publication
source-quote = Quote
source-statistic = Statistic
source-web = Web
source-prompt-source = Title, author, date
source-prompt-my_basis = What you base this on
source-prompt-assertion = Who says so, where
source-prompt-by_definition = Term and definition
source-prompt-case_study = The case, where, when
source-prompt-common_belief = Believed by whom
source-prompt-data = Dataset, owner, year
source-prompt-event = What, where, when
source-prompt-example = Describe the example
source-prompt-expert_opinion = Expert, field, venue
source-prompt-law = Law, article, country
source-prompt-media = Title, channel, date
source-prompt-personal_experience = What you lived, when
source-prompt-publication = Title, author, year
source-prompt-quote = The words, who said it
source-prompt-statistic = Figure, source, year
source-prompt-web = Page, site, date visited

## Status bar
status-boxes = { $count ->
    [one] { $count } box
   *[other] { $count } boxes
}
status-toggle_theme = Toggle theme
status-zoom = Zoom
status-zoom_value = { $percent }%
