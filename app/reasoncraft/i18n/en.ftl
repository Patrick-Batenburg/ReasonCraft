# ReasonCraft UI strings (English).
# Code uses dotted keys; each dot becomes a dash here (ribbon.tab.home -> ribbon-tab-home).

app-name = ReasonCraft
untitled = Untitled
window-title = { $doc } — { $app }

## Title bar
titlebar-search = Search commands ({ $keys })
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
group-layout = Layout
group-overview = Overview
group-debug = Debug
group-eval_claim = Claims and premises
group-eval_card = Cards
group-eval_source = Sources
group-eval_clear = Clear
group-eval_view = Display
group-evaluation = Evaluation
group-box = Box
group-add_source = Add source

## Commands
action-new = New
action-open = Open
action-save = Save
action-save_as = Save As
action-import_argdown = Import Argdown…
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
action-comment = Comment
action-connect = Connect to…
action-box_type = Type
action-link = Link
action-image = Image
action-remove_image = Remove image
action-delete = Delete
action-delete_branch = Delete branch
action-find = Find…
action-replace = Replace…
action-spelling = Spelling
action-zoom_fit = Fit
action-zoom_fit_selection = Fit selection
action-zoom_100 = 100%
action-zoom_in = Zoom in
action-zoom_out = Zoom out
action-panel_left = Building panel
action-panel_right = Text panel
action-next_pane = Next pane
action-prev_pane = Previous pane
action-minimap = Minimap
action-numbers = Numbers
minimap-auto = Show minimap when the map does not fit
minimap-on = Always show minimap
minimap-off = Hide minimap
minimap-name = Overview map
action-load_sample = Load sample map (debug)
action-source_icons = Source icons (debug)

## Search commands (commands that have no ribbon button of their own)
command-disabled = Not available right now
command-whole_card_reason = Reason for the whole card
command-whole_card_objection = Objection to the whole card
command-edit = Edit text
command-jump_parking = Jump to the parking area and back
command-move_left = Move left among siblings
command-move_right = Move right among siblings
command-rate_high = Rate high
command-rate_mid = Rate mid
command-rate_low = Rate low
command-rate_clear = Clear the selection's rating
command-find_next = Find next
command-find_prev = Find previous

## Find bar
find-query = Find in boxes, sources and notes
find-replacement = Replace with
find-match_case = Aa
find-match_case-tip = Match case
find-whole_word = W
find-whole_word-tip = Whole word
find-count = { $current } of { $total }
find-none = No results
find-prev = Previous
find-next = Next
find-replace = Replace
find-replace_all = Replace all
find-close = Close find
find-replaced = Replaced { $count }

## Spelling panel
spell-title = Spelling
spell-loading = Loading dictionary…
spell-checking = Checking…
spell-suggesting = …
spell-done = Spelling check complete
spell-no_suggestions = No suggestions
spell-change_to = Change to
spell-change = Change
spell-change_all = Change all
spell-ignore = Ignore
spell-ignore_all = Ignore all
spell-add = Add to dictionary
spell-close = Close
spell-close_name = Close spelling
spell-add_refused = This word cannot be added to the dictionary.
spell-load_failed = The spelling dictionary could not be loaded ({ $error }). Editing works as usual, without spellcheck.

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
eval-tip-claim = Accept, doubt or reject the selected claim or premise ({ $high }, { $mid }, { $low }; { $clear } clears)
eval-tip-card = Rate how strongly the selected card's premises carry its conclusion ({ $high }, { $mid }, { $low }; { $clear } clears)
eval-tip-source = Rate how reliable the selected source is ({ $high }, { $mid }, { $low }; { $clear } clears)
eval-tip-clear_selection = Clear the rating of the selected item ({ $clear })
eval-tip-clear_map = Clear every rating in the map ({ $clear_map })

## Files
file-filter = ReasonCraft map
file-images = Images
file-argdown = Argdown files
modal-ok = OK
unsaved-title = Save changes to { $name }?
unsaved-message = Your changes will be lost if you don't save them.
unsaved-save = Save
unsaved-save_changes = Save changes
unsaved-discard = Don't save
unsaved-cancel = Cancel
unsaved-waiting_images = Preparing image…
closing-exports-title = Finishing exports
closing-exports-message = The window closes as soon as the running exports are done.
closing-exports-waiting = Waiting for the exports…
closing-exports-quit = Quit anyway
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

## Argdown import
error-import-title = Could not import { $file }
argdown-error-io = The file could not be read: { $detail }
argdown-error-too_large = The file is too large to import (over 2 MB).
argdown-error-too_many_boxes = The file holds more than 5000 statements: too many for one map.
argdown-error-too_deep = Relations are nested too deeply (line { $line }).
argdown-error-empty = The file holds no statements or arguments.
argdown-report-title = Imported with changes
argdown-report-message = The map could not keep everything as the file wrote it:
argdown-report-group = { $text } ({ $count ->
    [one] line
   *[other] lines
} { $lines })
argdown-report-more = { $lines } and { $more } more
argdown-note-contradiction = A contradiction (><) was left out: maps have no contradictions.
argdown-note-extra_equivalent_wording = Another wording of a statement, or another description or premise list of an argument, was left out; the first one is used.
argdown-note-shared_statement_copied = A statement used in several places was copied there.
argdown-note-cycle_broken = A relation that would make a circle was left out.
argdown-note-unknown_reference = Titles defined nowhere (such as "{ $title }") became statements with the title as text.
argdown-note-intermediate_conclusion = An intermediate conclusion was left out.
argdown-note-inference_rule_dropped = An inference rule was left out.
argdown-note-tags_dropped = Tags (#…) were left out.
argdown-note-undercut_on_statement = An undercut of a statement was read as an objection.
argdown-note-empty_argument = An argument without premises or text was left out.
argdown-note-moved_to_parking = An item could not go where the file puts it and is in the parking area.
argdown-note-relation_dropped = A relation to a conclusion that was left out was left out too; its statement is in the parking area.
argdown-note-link_dropped = A link that is not a safe web or mail link was left out.
argdown-note-text_cut = Text longer than { $max } characters was cut to fit a box.

## Backstage
backstage-back = Back
backstage-settings = Settings
backstage-document = Document
backstage-export = Export…
backstage-recent = Recent
backstage-recent-title = Recent maps
backstage-recent-none = No recent maps yet. Maps you open or save are listed here.
backstage-recent-item = { $name } in { $folder }
backstage-new-title = New map
backstage-new-blank = Blank map
backstage-new-blank-hint = An empty claim, ready for a map of your own.
backstage-templates = Templates
backstage-examples = Example maps
template-blank_reasoning = Blank reasoning map
template-blank_reasoning-hint = A claim with one reason and one objection to fill in.
template-blank_grouping = Blank grouping map
template-blank_grouping-hint = A topic with three categories to fill in.
template-essay_plan = Essay plan
template-essay_plan-hint = A claim, three reasons and an objection with your answer, plus prompts for the essay.
template-peer_review = Peer review
template-peer_review-hint = A claim to review, with a note on the comment presets.
example-reasoning = Reasoning example
example-reasoning-hint = Should cities make public transport free? Reasons, objections, sources and ratings.
example-grouping = Grouping example
example-grouping-hint = Ways to reduce food waste, sorted into categories.
settings-title = Settings
settings-author = Your name
settings-author_hint = Shown on the comments you add
settings-theme = Theme
settings-checks = Checks
settings-checks_hint = Which hints the Check tab shows. Checks never stop you from doing anything.

settings-default_language = Language of new documents
settings-default_language_hint = Used by the language-aware checks. Change it for one document on the Document page.
settings-spellcheck = Check spelling in the document language
settings-spellcheck_hint = Underlines misspelled words while you type and enables Spelling (F7).
settings-autosave = Autosave every
settings-autosave_unit = { " " }seconds
settings-autosave_interval = Seconds between autosaves
settings-autosave_hint = Keeps a copy of unsaved work, so it can be restored after a crash. Your files are only written when you save.
settings-ui_scale = Interface size
settings-ui_scale_unit = { " " }%
settings-reset = Reset to defaults
settings-reset-hint = Puts every setting back as it was at the first start. Your personal dictionary is kept.
settings-reset-title = Reset all settings?
settings-reset-message = Every setting goes back to its default: theme, languages, interface size, checks, export choices and the rest. Your personal dictionary is kept.
settings-reset-cancel = Cancel
settings-reset-confirm = Reset

## Recovery
recovery-title = Restore unsaved work?
recovery-message = ReasonCraft did not close normally last time. These documents had unsaved changes:
recovery-entry = { $title } ({ $file }), autosaved { $time }
recovery-never_saved = never saved
recovery-restore_all = Restore all
recovery-discard_all = Discard all
recovery-later = Decide later
recovery-tab_title = { $name } (recovered)
recovery-original_changed = The file changed since you opened or last saved it. Replacing it overwrites those changes.
recovery-unavailable = Autosave is off: the recovery folder cannot be used.
recovery-session_gone = the recovery session folder is gone
recovery-write_failed = Autosave could not write a copy: { $detail }. It tries again at the next autosave.

## Document page
document-title = Document
document-title_field = Title
document-author_field = Author
document-language = Language
document-language_hint = Checks such as "and"/"or" in one statement use this language.
lang-en = English
lang-nl = Dutch

## Interface language (View ribbon, Language menu; settings page).
## Each language is named in its own language.
group-language = Language
action-ui_language = Language
ui_language-system = System
ui_language-en = English
ui_language-nl = Nederlands
settings-ui_language = Language of the interface
settings-ui_language_hint = System follows the language of your computer.

## Key names in shortcuts (Ctrl+Shift+R, Alt+Left)
key-ctrl = Ctrl
key-shift = Shift
key-alt = Alt
key-cmd = Cmd
key-opt = Option
key-control = Control
key-esc = Esc
key-enter = Enter
key-space = Space
key-tab = Tab
key-backspace = Backspace
key-delete = Delete
key-left = Left
key-right = Right
key-up = Up
key-down = Down

## Themes
theme-system = System
theme-light = Light
theme-dark = Dark

## Layout directions (View ribbon, Layout direction menu)
action-layout_direction = Layout direction
layout_direction-top_down = Top-down
layout_direction-bottom_up = Bottom-up
layout_direction-left_right = Left to right

## Connector styles (View ribbon, Connectors menu)
action-connector_style = Connectors
connector_style-curved = Curved
connector_style-elbow = Elbow
connector_style-tapered = Tapered

## Side panels
panel-building = Building
panel-coming_later = Coming in a later version.
text-tab-outline = Outline
text-tab-essay = Essay
text-tab-check = Check
text-tab-snippets = Snippets
essay-help-title = How to use
essay-help = The essay follows your map. The introduction states the claim. Then comes one section per branch, with the strongest argument first: its reasons and their evidence, then the counterarguments and how they are answered. The conclusion repeats the claim and the main reasons. Write a paragraph under each statement; click a statement to find it on the map.
essay-intro = Introduction
essay-intro-field = Introduction paragraph
essay-intro-hint = State the claim you will argue for
essay-conclusion = Conclusion
essay-conclusion-field = Conclusion paragraph
essay-conclusion-hint = Repeat the claim and the main reasons
essay-section = Section { $n }
essay-section_with = Section { $n }: { $premises }
essay-no_sections = Add reasons or objections under the claim: each one becomes a section of the essay.
essay-paragraph_for = Paragraph for: { $statement }
essay-hint = Write this part of the essay
essay-inactive = Not in the essay right now
essay-inactive-hint = Text written for items that are parked or no longer part of an argument. It is kept, and counts again when the item returns to the essay.
essay-unplaced = Unplaced notes
essay-unplaced-hint = Text written for parts of the map that were deleted. Copy it to where it belongs, or remove it.
essay-copy = Copy text
essay-remove = Remove note
essay-unplaced_note = Unplaced note { $n }
essay-unplaced_numbered = { $label } (note { $n })
essay-copy_name = Copy the text of { $name }
essay-remove_name = Remove { $name }
snippet-new = New snippet
snippet-tip-new = Add an empty snippet to type or paste text into
snippet-paste = Paste as snippet
snippet-tip-paste = Add the text on the clipboard as a new snippet ({ $paste } over this tab does the same)
snippet-empty = No snippets yet. Keep pieces of source text here, beside your map: paste text with { $paste } or use the buttons above.
snippet-hint = Type or paste text
snippet-text = Snippet text
snippet-delete = Delete snippet
snippet-drag = Drag to reorder; hold Alt to drag the whole snippet onto the map
snippet-drag_to_map = Drag to map: “{ $text }”
snippet-tip-drag_to_map = Drag the selected text onto the map; it keeps this snippet's source
snippet-source_type = Source type
snippet-citation = Citation (author, year, title)
snippet-link = Source link
snippet-capped = The text was too long: only the first { $max } characters were kept.
outline-toggle-numbering = Numbers
outline-toggle-labels = Labels
outline-toggle-indent = Indent
outline-toggle-colour = Colour
outline-toggle-notes = Notes
outline-tip-numbering = Show the numbers (1, 1a, 1.w1, P1)
outline-tip-labels = Show the linking words (because, but, however)
outline-tip-indent = Indent each line by its depth
outline-tip-colour = Mark each line with its role colour
outline-tip-notes = Include notes
outline-copy_all = Copy all
outline-tip-copy_all = Copy the outline as plain text
outline-copied = Copied
outline-empty = (empty)
check-all_clear = No problems found. Well done!
check-ignore = Ignore
check-ignore_name = Ignore { $kind } for { $item }
check-restore = Show ignored hints ({ $count })
check-heading = { $kind } ({ $count })
check-kind-key_term = Missing key words
check-kind-compound = Two statements in one
check-kind-question = Questions
check-kind-unsupported = Nothing supports it
check-kind-empty_category = Empty categories
check-kind-incomplete_evaluation = Not rated yet
check-kind-inconsistent_evaluation = Ratings that disagree
check-kind-not_connected = In the parking area
check-help-key_term = Every key word of a statement should come back in the premises of the reason or objection under it (the key-word rule): add a co-premise that covers the missing words. Short words in singular and plural (job, jobs) may be reported even when they are covered.
check-help-compound = Each box should say one thing. A statement joined by “{ $and }” or “{ $or }” may say two: if you could accept one part and reject the other, it is really two.
check-help-question = Claims and premises are statements: you can accept or reject them, but not a question.
check-help-unsupported = Every premise needs support: a reason or a source under it.
check-help-empty_category = A main category should have something under it.
check-help-incomplete_evaluation = Once you start rating, rate every statement, card and source.
check-help-inconsistent_evaluation = A rating should fit the ratings it rests on.
check-help-not_connected = Items in the parking area are not part of the argument yet.
check-explain-key_term = Missing: { $terms }
check-explain-compound = Does this say two things (joined by “{ $terms }”)? If you could accept one part and reject the other, split it.
check-explain-question = This is a question. Write it as a statement you can accept or reject.
check-explain-unsupported = Nothing supports this premise yet. Add a reason or a source under it.
check-explain-empty_category = This category has nothing under it. Add items or remove it.
check-explain-incomplete_evaluation = { $count ->
    [one] { $count } item is not rated yet, while other parts are. Click to go to the next one.
   *[other] { $count } items are not rated yet, while other parts are. Click to go to the next one.
}
check-explain-inconsistent_evaluation = { $case ->
    [strong] This card is rated strong, but all its premises are rejected.
    [solid] This source is rated solid, but the premise it supports is rejected.
   *[nil] This card is rated nil, but its premises are accepted and nothing objects to it.
}
check-explain-not_connected = This is in the parking area. Connect it to the map, or delete it.
check-card = { $kind }: { $premises }
building-boxes = Boxes
building-sources = Sources
building-comments = Comments
building-templates = Templates
building-item_name = { $item } (Building panel)

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
rating-strong-supports = strongly supports
rating-strong-opposes = strongly opposes
rating-strong-rebuts = strongly rebuts
rating-weak-supports = weakly supports
rating-weak-opposes = weakly opposes
rating-weak-rebuts = weakly rebuts
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
status-issues = { $count ->
    [0] No issues
    [one] { $count } issue
   *[other] { $count } issues
}
status-tip-issues = Show the issues in the Check tab
status-toggle_theme = Toggle theme
status-zoom = Zoom
status-zoom_value = { $percent }%
connect-hint = Click a box or card to connect the parked item to it, or select one and press Enter — Esc to cancel

## Comments
comment-anonymous = Anonymous
comment-title = Comments
comment-close = Close comments
comment-none = No comments yet.
comment-hint = Write a comment
comment-add = Add
comment-presets = Presets
comment-delete = Delete comment
comment-badge = { $count ->
    [one] { $count } comment
   *[other] { $count } comments
}
comment-verdict-none = None
comment-verdict-positive = Positive
comment-verdict-needs_work = Needs work
comment-verdict-problem = Problem
comment-preset-great_work = Great work.
comment-preset-good_structure = Clear, well-built structure.
comment-preset-strong_evidence = Well supported by evidence.
comment-preset-excellent_evaluation = Careful, well-judged ratings.
comment-preset-getting_there = Getting there.
comment-preset-refinement_needed = Could be sharper.
comment-preset-more_evidence_needed = Needs more evidence.
comment-preset-too_accepting = Accepted too easily.
comment-preset-unclear = Unclear: what is meant here?
comment-preset-rethink_structure = Rethink how this is built.
comment-preset-source_not_appropriate = This source does not fit.
comment-preset-inconsistent_evaluation = Ratings contradict each other.

## Box type and links
card_kind-reason = Reason
card_kind-objection = Objection
link-title = Link
link-url = Link address
link-hint = https://example.org/page
link-save = Save
link-save_link = Save link
link-cancel = Cancel
link-remove = Remove link
link-invalid = Enter a web address (https://…, or a domain such as example.org/page) or an e-mail link (mailto:…).
link-open = Open link: { $url }
link-open_failed = Could not open the link
link-open_failed_detail = { $url } could not be opened: { $detail }
link-unsafe = { $url } was not opened: only web links (http, https) and e-mail links (mailto) can be opened. Edit the link to fix it.
link-unsafe_label = Link that cannot be opened: { $url }

## Images in boxes
image-missing = Image missing
image-error-title = Could not use { $file }
image-error-too_large = The image is too large. Use an image under 40 MB and at most 12000 × 12000 pixels.
image-error-unsupported = This file is not a PNG, JPEG or WebP image.
image-error-corrupt = The image is damaged and cannot be read.
image-error-read = The file could not be read: { $detail }
image-error-stopped = Preparing the image stopped unexpectedly. Please try again.
image-preparing = Preparing image…
image-discarded = The image was not added: its box was deleted or can no longer hold an image.
export-error-too_large = This map is too large to export as an image ({ $detail } pixels). An image can have at most about 134 million pixels (512 MiB) and 16,384 pixels on a side. Use a lower scale, or export as SVG or PDF.
export-error-io = The file could not be written: { $detail }
export-error-render = The image could not be drawn: { $detail }
export-error-clipboard = The image could not be copied: { $detail }
export-error-clipboard_too_large = This map is too large to copy as an image ({ $detail } pixels). Export it as PNG, SVG or PDF instead.
export-error-pdf = The PDF could not be made: { $detail }
export-error-open = The PDF could not be opened for printing: { $detail }
export-error-title = Export failed
action-copy_image = Copy as image
group-image = Image
group-text = Text
group-print = Print
export-png = PNG
export-svg = SVG
export-pdf = PDF
export-outline = Outline
export-essay = Essay
export-page_setup = Page setup…
export-print = Print
export-format-png = PNG image
export-format-svg = SVG drawing
export-format-pdf = PDF document
export-format-outline_md = Markdown
export-format-outline_txt = Plain text
export-format-outline_docx = Word
export-format-essay_md = Markdown
export-format-essay_text = Plain text
export-format-essay_docx = Word
export-format-argdown = Argdown
export-argdown = Argdown
export-argdown_no_images = Images are not included in Argdown.
export-dialog-title = Export as { $format }
export-dialog-export = Export
export-dialog-ok = OK
export-dialog-cancel = Cancel
export-page_setup-title = Page setup
export-scope = What to export
export-scope-map = Whole map
export-scope-selection = Selection
export-theme = Colours
export-theme-light = Light theme
export-theme-current = Current theme
export-png-scale = Scale
export-png-scale_value = { $scale }×
export-png-transparent = Transparent background
export-svg-outlines = Text as outlines (looks the same without the font)
export-note-emoji = Emoji are left out when text is turned into outlines.
export-page-paper = Paper
export-page-a4 = A4
export-page-letter = Letter
export-page-orientation = Orientation
export-page-portrait = Portrait
export-page-landscape = Landscape
export-page-fit = Fit
export-page-one_page = One page
export-page-tile = Across pages at
export-page-margin = Margin
export-done = Export done.
export-open_folder = Open folder
export-copied = Copied to the clipboard.
status-exporting = Exporting…
status-preparing_image = Preparing image…
status-text_cut = Text was cut to 50 000 characters.
status-jobs-exports = { $count ->
    [one] One export running
   *[other] { $count } exports running
}
status-jobs-images = { $count ->
    [one] One image being prepared
   *[other] { $count } images being prepared
}
export-error-stopped = The export stopped unexpectedly. Please try again.
export-error-unsupported = This kind of export is not available yet ({ $detail }).

## Help (Text panel tab, Help ribbon tab, F1) and About
text-tab-help = Help
group-help = Help
group-examples = Example maps
help-cmd-topics = Help topics
help-cmd-keyboard = Keyboard shortcuts
help-cmd-about = About ReasonCraft
help-back = Back
help-contents = Contents
help-contents-concepts = Concepts
help-contents-howto = How to
help-check_link = Help on { $kind }
help-keyboard-table = The shortcuts on this computer
help-key-co_premise = Add a co-premise to the selected card
help-key-whole_card_reason = Add a reason for the whole card
help-key-whole_card_objection = Add an objection to the whole card
help-key-source = Add a source
help-key-edit = Edit the selected box
help-key-delete = Delete (what is under it moves to Not connected)
help-key-delete_branch = Delete with everything under it
help-key-jump_parking = Jump between the map and Not connected
help-key-comments = Open the comments of the selected box
help-key-connect = Connect to…: connect the selected parked item
help-key-move_left = Move the selection one place to the left
help-key-move_right = Move the selection one place to the right
help-key-move_up = Move the selection one place up
help-key-move_down = Move the selection one place down
help-key-rate_high = Rate high: accept, strong, solid
help-key-rate_mid = Rate mid: unsure, weak, shaky
help-key-rate_low = Rate low: reject, nil
help-key-rate_clear = Clear the selection's rating
help-key-clear_map = Clear every rating in the map
help-key-undo = Undo
help-key-redo = Redo
help-key-save = Save
help-key-save_as = Save as
help-key-open = Open
help-key-new = New map
help-key-zoom_100 = Zoom to 100%
help-key-zoom_fit = Show the whole map
help-key-zoom_fit_selection = Show the selection
help-key-zoom_in = Zoom in
help-key-zoom_out = Zoom out
help-key-search_commands = Search commands
help-key-find = Find
help-key-replace = Find and replace
help-key-find_next = Next hit
help-key-find_prev = Previous hit
help-key-spelling = Check spelling
help-key-next_pane = Next pane
help-key-prev_pane = Previous pane
help-key-help = Help for the part you are in
backstage-about = About
about-tagline = A free, open-source app for argument maps and grouping maps. It works fully offline.
about-licence = Licence: { $licence }, at your option.
about-credits = Spellcheck uses the English Hunspell dictionary from SCOWL by Kevin Atkinson and the Dutch dictionary of OpenTaal, as packaged by the dictionaries collection. Thank you to everyone who works on them.

## Accessibility: names screen readers announce
a11y-in_group = { $group }: { $name }
a11y-resize_building = Resize the Building panel
a11y-resize_text = Resize the Text panel

## Start-up: shown when the window cannot open
startup-failed-title = ReasonCraft could not start
startup-failed-body = ReasonCraft could not open its window. The graphics driver of this computer may be missing or out of date; updating it usually helps.
startup-failed-details = Technical details: { $details }
