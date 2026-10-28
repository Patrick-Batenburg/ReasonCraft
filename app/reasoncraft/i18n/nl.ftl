# ReasonCraft UI strings (Dutch).
# Exactly the keys of en.ftl, with the same placeables (test-enforced).
# Informal "je"; the fixed terms are listed in the step 6 task 4 report.

app-name = ReasonCraft
untitled = Naamloos
window-title = { $doc } — { $app }

## Title bar
titlebar-search = Opdrachten zoeken ({ $keys })
window-minimize = Minimaliseren
window-maximize = Maximaliseren
window-restore = Vorig formaat
window-close = Venster sluiten

## Document tabs
tabs-new = Nieuw tabblad
tabs-close = { $name } sluiten

## Ribbon tabs
ribbon-tab-file = Bestand
ribbon-tab-home = Start
ribbon-tab-insert = Invoegen
ribbon-tab-evaluate = Beoordelen
ribbon-tab-view = Beeld
ribbon-tab-export = Exporteren
ribbon-tab-help = Help
ribbon-scroll_left = Lint naar links schuiven
ribbon-scroll_right = Lint naar rechts schuiven
ribbon-group_more = { $group } (meer)

## Ribbon groups
group-clipboard = Klembord
group-add = Toevoegen
group-editing = Bewerken
group-zoom = Zoomen
group-theme = Thema
group-panels = Panelen
group-layout = Indeling
group-overview = Overzicht
group-debug = Debug
group-eval_claim = Beweringen en premissen
group-eval_card = Blokken
group-eval_source = Bronnen
group-eval_clear = Wissen
group-eval_view = Weergave
group-evaluation = Beoordeling
group-box = Vak
group-add_source = Bron toevoegen
group-boxes = Vakken
group-whole_card = Heel blok
group-sources = Bronnen

## Commands
action-new = Nieuw
action-open = Openen
action-save = Opslaan
action-save_as = Opslaan als
action-import_argdown = Argdown importeren…
action-undo = Ongedaan maken
action-redo = Opnieuw
action-paste = Plakken
action-cut = Knippen
action-copy = Kopiëren
action-add_reason = Reden
action-add_objection = Bezwaar
action-add_co_premise = Medepremisse
action-add_category = Categorie
action-add_note = Notitie
action-add_source = Bron
action-comment = Opmerking
action-connect = Verbinden…
action-box_type = Soort
action-link = Link
action-image = Afbeelding
action-remove_image = Afbeelding wissen
action-delete = Verwijderen
action-delete_branch = Tak verwijderen
action-find = Zoeken…
action-replace = Vervangen…
action-spelling = Spelling
action-zoom_fit = Alles tonen
action-zoom_fit_selection = Selectie tonen
action-zoom_100 = 100%
action-zoom_in = Inzoomen
action-zoom_out = Uitzoomen
action-panel_left = Bouwstenen
action-panel_right = Tekstpaneel
action-next_pane = Volgend paneel
action-prev_pane = Vorig paneel
action-minimap = Minikaart
action-numbers = Nummers
minimap-auto = Minikaart tonen als de kaart niet past
minimap-on = Minikaart altijd tonen
minimap-off = Minikaart verbergen
minimap-name = Overzichtskaart
action-load_sample = Voorbeeldkaart laden (debug)
action-source_icons = Bronpictogrammen (debug)

## Search commands (commands that have no ribbon button of their own)
command-disabled = Nu niet beschikbaar
command-whole_card_reason = Reden voor het hele blok
command-whole_card_objection = Bezwaar tegen het hele blok
command-edit = Tekst bewerken
command-jump_parking = Naar Niet verbonden en terug
command-move_left = Eén plek naar links
command-move_right = Eén plek naar rechts
command-rate_high = Beoordeling: hoog
command-rate_mid = Beoordeling: midden
command-rate_low = Beoordeling: laag
command-rate_clear = Beoordeling van de selectie wissen
command-find_next = Volgende zoeken
command-find_prev = Vorige zoeken

## Find bar
find-query = Zoeken in vakken, bronnen en notities
find-replacement = Vervangen door
find-match_case = Aa
find-match_case-tip = Hoofdlettergevoelig
find-whole_word = W
find-whole_word-tip = Hele woorden
find-count = { $current } van { $total }
find-none = Geen resultaten
find-prev = Vorige
find-next = Volgende
find-replace = Vervangen
find-replace_all = Alles vervangen
find-close = Zoeken sluiten
find-replaced = { $count } vervangen

## Spelling panel
spell-title = Spelling
spell-loading = Woordenlijst laden…
spell-checking = Controleren…
spell-suggesting = …
spell-done = Spellingcontrole klaar
spell-no_suggestions = Geen suggesties
spell-change_to = Wijzigen in
spell-change = Wijzigen
spell-change_all = Alles wijzigen
spell-ignore = Negeren
spell-ignore_all = Alles negeren
spell-add = Aan woordenlijst toevoegen
spell-close = Sluiten
spell-close_name = Spelling sluiten
spell-add_refused = Dit woord kan niet aan de woordenlijst worden toegevoegd.
spell-load_failed = De woordenlijst kon niet worden geladen ({ $error }). Bewerken werkt gewoon, alleen zonder spellingcontrole.

## Evaluation
eval-accept = Aanvaarden
eval-unsure = Betwijfelen
eval-reject = Verwerpen
eval-strong = Sterk
eval-weak = Zwak
eval-nil = Geen
eval-solid = Solide
eval-shaky = Wankel
eval-clear_selection = Selectie
eval-clear_map = Kaart
eval-show_markers = Markeringen
eval-shading = Kleuring
eval-tip-claim = Aanvaard, betwijfel of verwerp de gekozen bewering of premisse ({ $high }, { $mid }, { $low }; { $clear } wist)
eval-tip-card = Beoordeel hoe sterk de premissen van het gekozen blok de conclusie dragen ({ $high }, { $mid }, { $low }; { $clear } wist)
eval-tip-source = Beoordeel hoe betrouwbaar de gekozen bron is ({ $high }, { $mid }, { $low }; { $clear } wist)
eval-tip-clear_selection = Wis de beoordeling van het gekozen onderdeel ({ $clear })
eval-tip-clear_map = Wis alle beoordelingen op de kaart ({ $clear_map })

## Files
file-filter = ReasonCraft-kaart
file-images = Afbeeldingen
file-argdown = Argdown-bestanden
modal-ok = OK
unsaved-title = Wijzigingen in { $name } opslaan?
unsaved-message = Je wijzigingen gaan verloren als je ze niet opslaat.
unsaved-save = Opslaan
unsaved-save_changes = Wijzigingen opslaan
unsaved-discard = Niet opslaan
unsaved-cancel = Annuleren
unsaved-waiting_images = Afbeelding voorbereiden…
closing-exports-title = Exports afronden
closing-exports-message = Het venster sluit zodra de lopende exports klaar zijn.
closing-exports-waiting = Wachten op de exports…
closing-exports-quit = Toch afsluiten
replace-title = { $file } bestaat al. Vervangen?
replace-message = De huidige inhoud wordt overschreven.
replace-confirm = Vervangen
replace-cancel = Annuleren
error-open-title = { $file } kon niet worden geopend
error-save-title = { $file } kon niet worden opgeslagen
error-open-not_a_file = Dit is geen ReasonCraft-kaart.
error-open-missing_document = Dit ReasonCraft-bestand bevat geen kaart.
error-open-malformed = Dit ReasonCraft-bestand is beschadigd en kan niet worden gelezen. ({ $detail })
error-open-newer_version = Deze kaart is gemaakt met een nieuwere versie van ReasonCraft (bestandsversie { $found }; deze versie leest tot en met versie { $supported }). Werk ReasonCraft bij om hem te openen.
error-open-io = Het bestand kon niet worden gelezen: { $detail }
error-save-io = Het bestand kon niet worden geschreven: { $detail }. Je wijzigingen staan nog open en zijn niet opgeslagen.
error-save-other = De kaart kon niet worden opgeslagen: { $detail }. Je wijzigingen staan nog open en zijn niet opgeslagen.

## Argdown import
error-import-title = { $file } kon niet worden geïmporteerd
argdown-error-io = Het bestand kon niet worden gelezen: { $detail }
argdown-error-too_large = Het bestand is te groot om te importeren (meer dan 2 MB).
argdown-error-too_many_boxes = Het bestand bevat meer dan 5000 uitspraken: te veel voor één kaart.
argdown-error-too_deep = Relaties zijn te diep genest (regel { $line }).
argdown-error-empty = Het bestand bevat geen uitspraken of argumenten.
argdown-report-title = Geïmporteerd met wijzigingen
argdown-report-message = De kaart kon niet alles overnemen zoals het in het bestand stond:
argdown-report-group = { $text } ({ $count ->
    [one] regel
   *[other] regels
} { $lines })
argdown-report-more = { $lines } en nog { $more }
argdown-note-contradiction = Een tegenspraak (><) is weggelaten: kaarten kennen geen tegenspraak.
argdown-note-extra_equivalent_wording = Een andere formulering van een uitspraak, of een andere beschrijving of premisselijst van een argument, is weggelaten; de eerste wordt gebruikt.
argdown-note-shared_statement_copied = Een uitspraak die op meerdere plaatsen voorkomt, is naar elk van die plaatsen gekopieerd.
argdown-note-cycle_broken = Een relatie die een kringredenering zou maken, is weggelaten.
argdown-note-unknown_reference = Titels die nergens zijn gedefinieerd (zoals "{ $title }"), zijn uitspraken geworden met de titel als tekst.
argdown-note-intermediate_conclusion = Een tussenconclusie is weggelaten.
argdown-note-inference_rule_dropped = Een afleidingsregel is weggelaten.
argdown-note-tags_dropped = Tags (#…) zijn weggelaten.
argdown-note-undercut_on_statement = Een ondermijning van een uitspraak is gelezen als bezwaar.
argdown-note-empty_argument = Een argument zonder premissen of tekst is weggelaten.
argdown-note-moved_to_parking = Een onderdeel kon niet op de plek uit het bestand komen en staat nu bij Niet verbonden.
argdown-note-relation_dropped = Een relatie naar een weggelaten conclusie is ook weggelaten; de uitspraak staat nu bij Niet verbonden.
argdown-note-link_dropped = Een link die geen veilige web- of e-maillink is, is weggelaten.
argdown-note-text_cut = Tekst van meer dan { $max } tekens is ingekort zodat hij in een vak past.

## Backstage
backstage-back = Terug
backstage-settings = Instellingen
backstage-document = Document
backstage-export = Exportformaten
backstage-recent = Recent
backstage-recent-title = Recente kaarten
backstage-recent-none = Nog geen recente kaarten. Kaarten die je opent of opslaat, komen hier te staan.
backstage-recent-item = { $name } in map { $folder }
backstage-new-title = Nieuwe kaart
backstage-new-blank = Lege kaart
backstage-new-blank-hint = Een lege bewering, klaar voor je eigen kaart.
backstage-templates = Sjablonen
backstage-examples = Voorbeeldkaarten
template-blank_reasoning = Lege argumentkaart
template-blank_reasoning-hint = Een bewering met een reden en een bezwaar om in te vullen.
template-blank_grouping = Lege groeperingskaart
template-blank_grouping-hint = Een onderwerp met drie categorieën om in te vullen.
template-essay_plan = Betoogplan
template-essay_plan-hint = Een bewering, drie redenen en een bezwaar met je antwoord, plus aanwijzingen voor het betoog.
template-peer_review = Feedback geven
template-peer_review-hint = Een bewering om te beoordelen, met een notitie over de standaardopmerkingen.
example-reasoning = Voorbeeld argumentkaart
example-reasoning-hint = Moeten steden het openbaar vervoer gratis maken? Redenen, bezwaren, bronnen en beoordelingen.
example-grouping = Voorbeeld groeperingskaart
example-grouping-hint = Manieren om voedselverspilling te verminderen, ingedeeld in categorieën.
settings-title = Instellingen
settings-author = Je naam
settings-author_hint = Staat bij de opmerkingen die je toevoegt
settings-theme = Thema
settings-checks = Controles
settings-checks_hint = Welke tips het tabblad Controle toont. Controles houden je nooit tegen.

settings-default_language = Taal van nieuwe documenten
settings-default_language_hint = Gebruikt door de taalgevoelige controles. Wijzig de taal van één document op de pagina Document.
settings-default_language-ui = Zelfde als de interface
settings-spellcheck = Spelling controleren in de taal van het document
settings-spellcheck_hint = Onderstreept fout gespelde woorden terwijl je typt en zet Spelling (F7) aan.
settings-autosave = Automatisch opslaan elke
settings-autosave_unit = { " " }seconden
settings-autosave_interval = Seconden tussen automatisch opslaan
settings-autosave_hint = Bewaart een kopie van niet-opgeslagen werk, zodat je het na een crash kunt herstellen. Je bestanden worden alleen geschreven als je opslaat.
settings-ui_scale = Grootte van de interface
settings-ui_scale_unit = { " " }%
settings-reset = Standaardinstellingen herstellen
settings-reset-hint = Zet alle instellingen terug zoals bij de eerste start. Je eigen woordenlijst blijft bewaard.
settings-reset-title = Alle instellingen herstellen?
settings-reset-message = Elke instelling gaat terug naar de standaardwaarde: thema, talen, grootte van de interface, controles, exportkeuzes en de rest. Je eigen woordenlijst blijft bewaard.
settings-reset-cancel = Annuleren
settings-reset-confirm = Herstellen

## Recovery
recovery-title = Niet-opgeslagen werk herstellen?
recovery-message = ReasonCraft is de vorige keer niet normaal afgesloten. Deze documenten hadden niet-opgeslagen wijzigingen:
recovery-entry = { $title } ({ $file }), automatisch opgeslagen { $time }
recovery-never_saved = nooit opgeslagen
recovery-restore_all = Alles herstellen
recovery-discard_all = Alles weggooien
recovery-later = Later beslissen
recovery-tab_title = { $name } (hersteld)
recovery-original_changed = Het bestand is gewijzigd sinds je het opende of voor het laatst opsloeg. Vervangen overschrijft die wijzigingen.
recovery-unavailable = Automatisch opslaan staat uit: de herstelmap kan niet worden gebruikt.
recovery-session_gone = de map van de herstelsessie is verdwenen
recovery-write_failed = Automatisch opslaan kon geen kopie schrijven: { $detail }. Bij de volgende keer automatisch opslaan wordt het opnieuw geprobeerd.

## Document page
document-title = Document
document-title_field = Titel
document-author_field = Auteur
document-language = Taal
document-language_hint = Controles zoals "en"/"of" in één uitspraak gebruiken deze taal.
lang-en = Engels
lang-nl = Nederlands

## Interface language (View ribbon, Language menu; settings page).
## Each language is named in its own language.
group-language = Taal
action-ui_language = Taal
ui_language-system = Systeem
ui_language-en = English
ui_language-nl = Nederlands
settings-ui_language = Taal van de interface
settings-ui_language_hint = Systeem volgt de taal van je computer.

## Key names in shortcuts (Ctrl+Shift+R, Alt+Left)
key-ctrl = Ctrl
key-shift = Shift
key-alt = Alt
key-cmd = Cmd
key-opt = Option
key-control = Control
key-esc = Esc
key-enter = Enter
key-space = Spatie
key-tab = Tab
key-backspace = Backspace
key-delete = Delete
key-left = Links
key-right = Rechts
key-up = Omhoog
key-down = Omlaag

## Themes
theme-system = Systeem
theme-light = Licht
theme-dark = Donker

## Layout directions (View ribbon, Layout direction menu)
action-layout_direction = Richting
layout_direction-top_down = Van boven naar beneden
layout_direction-bottom_up = Van beneden naar boven
layout_direction-left_right = Van links naar rechts

## Connector styles (View ribbon, Connectors menu)
action-connector_style = Lijnen
connector_style-curved = Gebogen
connector_style-elbow = Haaks
connector_style-tapered = Taps toelopend

## Side panels
panel-building = Bouwstenen
text-tab-outline = Opzet
text-tab-essay = Betoog
text-tab-check = Controle
text-tab-snippets = Fragmenten
essay-help-title = Zo werkt het
essay-help = Het betoog volgt je kaart. De inleiding noemt de bewering. Dan volgt één deel per tak, met het sterkste argument eerst: de redenen en hun onderbouwing, daarna de bezwaren en hoe die worden weerlegd. De conclusie herhaalt de bewering en de belangrijkste redenen. Schrijf een alinea onder elke uitspraak; klik op een uitspraak om hem op de kaart te vinden.
essay-intro = Inleiding
essay-intro-field = Alinea van de inleiding
essay-intro-hint = Noem de bewering die je gaat verdedigen
essay-conclusion = Conclusie
essay-conclusion-field = Alinea van de conclusie
essay-conclusion-hint = Herhaal de bewering en de belangrijkste redenen
essay-section = Deel { $n }
essay-section_with = Deel { $n }: { $premises }
essay-no_sections = Zet redenen of bezwaren onder de bewering: elk wordt een deel van het betoog.
essay-paragraph_for = Alinea bij: { $statement }
essay-hint = Schrijf dit deel van het betoog
essay-inactive = Nu niet in het betoog
essay-inactive-hint = Tekst bij onderdelen die niet verbonden zijn of niet meer bij een argument horen. Hij blijft bewaard en telt weer mee als het onderdeel terugkomt in het betoog.
essay-unplaced = Losse aantekeningen
essay-unplaced-hint = Tekst bij delen van de kaart die zijn verwijderd. Kopieer hem naar de juiste plek, of verwijder hem.
essay-copy = Tekst kopiëren
essay-remove = Aantekening verwijderen
essay-unplaced_note = Losse aantekening { $n }
essay-unplaced_numbered = { $label } (aantekening { $n })
essay-copy_name = Tekst van { $name } kopiëren
essay-remove_name = { $name } verwijderen
snippet-new = Nieuw fragment
snippet-tip-new = Voeg een leeg fragment toe om tekst in te typen of te plakken
snippet-paste = Plakken als fragment
snippet-tip-paste = Voeg de tekst op het klembord toe als nieuw fragment ({ $paste } boven dit tabblad doet hetzelfde)
snippet-empty = Nog geen fragmenten. Bewaar hier stukken brontekst, naast je kaart: plak tekst met { $paste } of gebruik de knoppen hierboven.
snippet-hint = Typ of plak tekst
snippet-text = Tekst van het fragment
snippet-delete = Fragment verwijderen
snippet-drag = Sleep om de volgorde te wijzigen; houd Alt ingedrukt om het hele fragment naar de kaart te slepen
snippet-drag_to_map = Naar de kaart slepen: “{ $text }”
snippet-tip-drag_to_map = Sleep de gekozen tekst naar de kaart; hij behoudt de bron van dit fragment
snippet-source_type = Soort bron
snippet-citation = Bronvermelding (auteur, jaar, titel)
snippet-link = Link naar de bron
snippet-capped = De tekst was te lang: alleen de eerste { $max } tekens zijn bewaard.
outline-toggle-numbering = Nummers
outline-toggle-labels = Signaalwoorden
outline-toggle-indent = Inspringen
outline-toggle-colour = Kleur
outline-toggle-notes = Notities
outline-tip-numbering = Toon de nummers (1, 1a, 1.w1, P1)
outline-tip-labels = Toon de signaalwoorden ({ $words })
outline-tip-indent = Laat elke regel inspringen naar zijn diepte
outline-tip-colour = Markeer elke regel met de kleur van zijn rol
outline-tip-notes = Notities opnemen
outline-copy_all = Alles kopiëren
outline-tip-copy_all = Kopieer de opzet als platte tekst
outline-copied = Gekopieerd
outline-empty = (leeg)
check-all_clear = Geen tips: alles ziet er goed uit. Goed gedaan!
check-ignore = Negeren
check-ignore_name = { $kind } negeren bij { $item }
check-restore = Genegeerde tips tonen ({ $count })
check-heading = { $kind } ({ $count })
check-kind-key_term = Ontbrekende kernwoorden
check-kind-compound = Twee uitspraken in één
check-kind-question = Vragen
check-kind-unsupported = Nog niet onderbouwd
check-kind-empty_category = Lege categorieën
check-kind-incomplete_evaluation = Nog niet beoordeeld
check-kind-inconsistent_evaluation = Beoordelingen die botsen
check-kind-not_connected = Niet verbonden
check-help-key_term = Elk kernwoord van een uitspraak hoort terug te komen in de premissen van de reden of het bezwaar eronder (de kernwoordregel): voeg een medepremisse toe die de ontbrekende woorden dekt. Korte woorden in enkelvoud en meervoud (baan, banen) kunnen gemeld worden, ook als ze gedekt zijn.
check-help-compound = Elk vak zegt één ding. Een uitspraak met “{ $and }” of “{ $or }” erin kan er twee zeggen: als je het ene deel kunt aanvaarden en het andere verwerpen, zijn het er eigenlijk twee.
check-help-question = Beweringen en premissen zijn uitspraken: je kunt ze aanvaarden of verwerpen, maar een vraag niet.
check-help-unsupported = Elke premisse heeft steun nodig: een reden of een bron eronder.
check-help-empty_category = Een hoofdcategorie hoort iets onder zich te hebben.
check-help-incomplete_evaluation = Ben je eenmaal begonnen met beoordelen, beoordeel dan elke uitspraak, elk blok en elke bron.
check-help-inconsistent_evaluation = Een beoordeling hoort te passen bij de beoordelingen waarop hij rust.
check-help-not_connected = Onderdelen die nog niet verbonden zijn, horen nog niet bij het argument.
check-explain-key_term = Ontbreekt: { $terms }
check-explain-compound = Zegt dit twee dingen (verbonden door “{ $terms }”)? Als je het ene deel kunt aanvaarden en het andere verwerpen, splits het dan.
check-explain-question = Dit is een vraag. Schrijf het als een uitspraak die je kunt aanvaarden of verwerpen.
check-explain-unsupported = Nog niets ondersteunt deze premisse. Zet er een reden of een bron onder.
check-explain-empty_category = Er staat niets onder deze categorie. Voeg onderdelen toe of verwijder de categorie.
check-explain-incomplete_evaluation = { $count ->
    [one] { $count } onderdeel is nog niet beoordeeld, terwijl andere dat wel zijn. Klik om naar het volgende te gaan.
   *[other] { $count } onderdelen zijn nog niet beoordeeld, terwijl andere dat wel zijn. Klik om naar het volgende te gaan.
}
check-explain-inconsistent_evaluation = { $case ->
    [strong] Dit blok is als sterk beoordeeld, maar al zijn premissen zijn verworpen.
    [solid] Deze bron is als solide beoordeeld, maar de premisse die door deze bron wordt gesteund, is verworpen.
   *[nil] Dit blok is als “geen” beoordeeld, maar zijn premissen zijn aanvaard en niets spreekt het tegen.
}
check-explain-not_connected = Dit is nog niet verbonden. Verbind het met de kaart, of verwijder het.
check-card = { $kind }: { $premises }
building-boxes = Vakken
building-sources = Bronnen
building-comments = Opmerkingen
building-templates = Sjablonen
building-item_name = { $item } (Bouwstenen)

## Canvas
canvas-empty_claim = Typ je hoofdbewering
canvas-label = Argumentkaart
editor-hint = Typ hier
plus-open = Onderdeel toevoegen
plus-whole_card_reason = Reden voor het hele blok
plus-whole_card_objection = Bezwaar tegen het hele blok
plus-source = Bron
parking-title = Niet verbonden

## Card roles
role-supports = steunt
role-opposes = spreekt tegen
role-rebuts = weerlegt

## Card rating labels
rating-strong-supports = steunt sterk
rating-strong-opposes = spreekt sterk tegen
rating-strong-rebuts = weerlegt sterk
rating-weak-supports = steunt zwak
rating-weak-opposes = spreekt zwak tegen
rating-weak-rebuts = weerlegt zwak
rating-nil-supports = steunt niet
rating-nil-opposes = spreekt niet tegen
rating-nil-rebuts = weerlegt niet

## Source types
source-source = Bron
source-my_basis = Eigen kennis
source-assertion = Mededeling
source-by_definition = Per definitie
source-case_study = Casus
source-common_belief = Algemene opvatting
source-data = Gegevens
source-event = Gebeurtenis
source-example = Voorbeeld
source-expert_opinion = Deskundige
source-law = Wet
source-media = Media
source-personal_experience = Persoonlijke ervaring
source-publication = Publicatie
source-quote = Citaat
source-statistic = Statistiek
source-web = Web
source-prompt-source = Titel, auteur, datum
source-prompt-my_basis = Waar je dit op baseert
source-prompt-assertion = Wie zegt het, waar
source-prompt-by_definition = Begrip en definitie
source-prompt-case_study = De casus, waar, wanneer
source-prompt-common_belief = Wie gelooft dit
source-prompt-data = Gegevensset, eigenaar, jaar
source-prompt-event = Wat, waar, wanneer
source-prompt-example = Beschrijf het voorbeeld
source-prompt-expert_opinion = Deskundige, vakgebied, plek
source-prompt-law = Wet, artikel, land
source-prompt-media = Titel, kanaal, datum
source-prompt-personal_experience = Wat je meemaakte, wanneer
source-prompt-publication = Titel, auteur, jaar
source-prompt-quote = De woorden, wie het zei
source-prompt-statistic = Cijfer, bron, jaar
source-prompt-web = Pagina, site, datum bezocht

## Status bar
status-boxes = { $count ->
    [one] { $count } vak
   *[other] { $count } vakken
}
status-issues = { $count ->
    [0] Geen tips
    [one] { $count } tip
   *[other] { $count } tips
}
status-tip-issues = Toon de tips in het tabblad Controle
status-toggle_theme = Thema wisselen
status-zoom = Zoom
status-zoom_value = { $percent }%
connect-hint = Klik op een vak of blok om het niet-verbonden onderdeel ermee te verbinden, of kies er een en druk op Enter — Esc om te annuleren

## Comments
comment-anonymous = Anoniem
comment-title = Opmerkingen
comment-close = Opmerkingen sluiten
comment-none = Nog geen opmerkingen.
comment-hint = Schrijf een opmerking
comment-add = Toevoegen
comment-presets = Standaardopmerkingen
comment-delete = Opmerking verwijderen
comment-badge = { $count ->
    [one] { $count } opmerking
   *[other] { $count } opmerkingen
}
comment-verdict-none = Geen
comment-verdict-positive = Positief
comment-verdict-needs_work = Kan beter
comment-verdict-problem = Probleem
comment-preset-great_work = Goed gedaan.
comment-preset-good_structure = Duidelijke, goed gebouwde structuur.
comment-preset-strong_evidence = Goed onderbouwd met bewijs.
comment-preset-excellent_evaluation = Zorgvuldige, goed afgewogen beoordelingen.
comment-preset-getting_there = Je bent op de goede weg.
comment-preset-refinement_needed = Kan scherper.
comment-preset-more_evidence_needed = Meer bewijs nodig.
comment-preset-too_accepting = Te snel aanvaard.
comment-preset-unclear = Onduidelijk: wat bedoel je hier?
comment-preset-rethink_structure = Denk opnieuw na over de opbouw.
comment-preset-source_not_appropriate = Deze bron past niet.
comment-preset-inconsistent_evaluation = De beoordelingen spreken elkaar tegen.

## Box type and links
card_kind-reason = Reden
card_kind-objection = Bezwaar
link-title = Link
link-url = Linkadres
link-hint = https://example.org/page
link-save = Opslaan
link-save_link = Link opslaan
link-cancel = Annuleren
link-remove = Link verwijderen
link-invalid = Vul een webadres in (https://…, of een domein zoals example.org/page) of een e-maillink (mailto:…).
link-open = Link openen: { $url }
link-open_failed = De link kon niet worden geopend
link-open_failed_detail = { $url } kon niet worden geopend: { $detail }
link-unsafe = { $url } is niet geopend: alleen weblinks (http, https) en e-maillinks (mailto) kunnen worden geopend. Bewerk de link om dit op te lossen.
link-unsafe_label = Link die niet kan worden geopend: { $url }

## Images in boxes
image-missing = Afbeelding ontbreekt
image-error-title = { $file } kan niet worden gebruikt
image-error-too_large = De afbeelding is te groot. Gebruik een afbeelding kleiner dan 40 MB en hooguit 12000 × 12000 pixels.
image-error-unsupported = Dit bestand is geen PNG-, JPEG- of WebP-afbeelding.
image-error-corrupt = De afbeelding is beschadigd en kan niet worden gelezen.
image-error-read = Het bestand kon niet worden gelezen: { $detail }
image-error-stopped = Het voorbereiden van de afbeelding is onverwacht gestopt. Probeer het opnieuw.
image-preparing = Afbeelding voorbereiden…
image-discarded = De afbeelding is niet toegevoegd: het vak is verwijderd of kan geen afbeelding meer bevatten.
export-error-too_large = Deze kaart is te groot om als afbeelding te exporteren ({ $detail } pixels). Een afbeelding mag hooguit ongeveer 134 miljoen pixels (512 MiB) hebben en 16.384 pixels per zijde. Kies een kleinere schaal, of exporteer als SVG of PDF.
export-error-io = Het bestand kon niet worden geschreven: { $detail }
export-error-render = De afbeelding kon niet worden getekend: { $detail }
export-error-clipboard = De afbeelding kon niet worden gekopieerd: { $detail }
export-error-clipboard_too_large = Deze kaart is te groot om als afbeelding te kopiëren ({ $detail } pixels). Exporteer hem als PNG, SVG of PDF.
export-error-pdf = De pdf kon niet worden gemaakt: { $detail }
export-error-open = De pdf kon niet worden geopend om af te drukken: { $detail }
export-error-title = Exporteren mislukt
action-copy_image = Kopiëren als beeld
group-image = Afbeelding
group-text = Tekst
group-print = Afdrukken
export-png = PNG
export-svg = SVG
export-pdf = PDF
export-outline = Opzet
export-essay = Betoog
export-page_setup = Pagina-instelling…
export-print = Afdrukken
export-format-png = PNG-afbeelding
export-format-svg = SVG-tekening
export-format-pdf = PDF-document
export-format-outline_md = Markdown
export-format-outline_txt = Platte tekst
export-format-outline_docx = Word
export-format-essay_md = Markdown
export-format-essay_text = Platte tekst
export-format-essay_docx = Word
export-format-argdown = Argdown
export-argdown = Argdown
export-argdown_no_images = Afbeeldingen gaan niet mee naar Argdown.
export-dialog-title = Exporteren als { $format }
export-dialog-export = Exporteren
export-dialog-ok = OK
export-dialog-cancel = Annuleren
export-page_setup-title = Pagina-instelling
export-scope = Wat wil je exporteren?
export-scope-map = Hele kaart
export-scope-selection = Selectie
export-theme = Kleuren
export-theme-light = Licht thema
export-theme-current = Huidig thema
export-png-scale = Schaal
export-png-scale_value = { $scale }×
export-png-transparent = Doorzichtige achtergrond
export-svg-outlines = Tekst als contouren (ziet er ook zonder het lettertype hetzelfde uit)
export-note-emoji = Emoji worden weggelaten als tekst in contouren wordt omgezet.
export-page-paper = Papier
export-page-a4 = A4
export-page-letter = Letter
export-page-orientation = Afdrukstand
export-page-portrait = Staand
export-page-landscape = Liggend
export-page-fit = Passend
export-page-one_page = Eén pagina
export-page-tile = Verdeeld over pagina's op
export-page-margin = Marge
export-done = Export klaar.
export-open_folder = Map openen
export-copied = Gekopieerd naar het klembord.
status-exporting = Exporteren…
status-preparing_image = Afbeelding voorbereiden…
status-saving = Opslaan…
status-text_cut = Tekst is ingekort tot 50 000 tekens.
status-open_cut = Een deel van dit bestand was te groot of onveilig: lange teksten zijn ingekort en de rest is weggelaten.
status-jobs-exports = { $count ->
    [one] Eén export bezig
   *[other] { $count } exports bezig
}
status-jobs-images = { $count ->
    [one] Eén afbeelding in voorbereiding
   *[other] { $count } afbeeldingen in voorbereiding
}
export-error-stopped = Het exporteren is onverwacht gestopt. Probeer het opnieuw.

## Help (Text panel tab, Help ribbon tab, F1) and About
text-tab-help = Help
group-help = Help
group-examples = Voorbeeldkaarten
help-cmd-topics = Helponderwerpen
help-cmd-keyboard = Sneltoetsen
help-cmd-about = Over ReasonCraft
help-back = Terug
help-contents = Inhoud
help-contents-concepts = Begrippen
help-contents-howto = Zo doe je het
help-check_link = Hulp bij { $kind }
help-keyboard-table = De sneltoetsen op deze computer
help-key-co_premise = Een medepremisse toevoegen aan het gekozen blok
help-key-whole_card_reason = Een reden voor het hele blok toevoegen
help-key-whole_card_objection = Een bezwaar tegen het hele blok toevoegen
help-key-source = Een bron toevoegen
help-key-edit = Het gekozen vak bewerken
help-key-delete = Verwijderen (wat eronder staat, gaat naar Niet verbonden)
help-key-delete_branch = Verwijderen met alles eronder
help-key-jump_parking = Heen en weer tussen de kaart en Niet verbonden
help-key-comments = De opmerkingen bij het gekozen vak openen
help-key-connect = Verbinden…: het gekozen losse onderdeel verbinden
help-key-move_left = De selectie één plek naar links schuiven
help-key-move_right = De selectie één plek naar rechts schuiven
help-key-move_up = De selectie één plek omhoog schuiven
help-key-move_down = De selectie één plek omlaag schuiven
help-key-rate_high = Hoog beoordelen: aanvaarden, sterk, solide
help-key-rate_mid = Midden beoordelen: betwijfelen, zwak, wankel
help-key-rate_low = Laag beoordelen: verwerpen, geen
help-key-rate_clear = De beoordeling van de selectie wissen
help-key-clear_map = Alle beoordelingen op de kaart wissen
help-key-undo = Ongedaan maken
help-key-redo = Opnieuw
help-key-save = Opslaan
help-key-save_as = Opslaan als
help-key-open = Openen
help-key-new = Nieuwe kaart
help-key-zoom_100 = Zoomen naar 100%
help-key-zoom_fit = De hele kaart tonen
help-key-zoom_fit_selection = De selectie tonen
help-key-zoom_in = Inzoomen
help-key-zoom_out = Uitzoomen
help-key-search_commands = Opdrachten zoeken
help-key-find = Zoeken
help-key-replace = Zoeken en vervangen
help-key-find_next = Volgende treffer
help-key-find_prev = Vorige treffer
help-key-spelling = Spelling controleren
help-key-next_pane = Volgend paneel
help-key-prev_pane = Vorig paneel
help-key-help = Hulp bij het deel waar je bent
backstage-about = Over
about-tagline = Een gratis opensource-app voor argumentkaarten en groeperingskaarten. Hij werkt volledig offline.
about-licence = Licentie: { $licence }, naar keuze.
about-credits = De spellingcontrole gebruikt het Engelse Hunspell-woordenboek uit SCOWL van Kevin Atkinson en het Nederlandse woordenboek van OpenTaal, zoals gebundeld in het project dictionaries. Dank aan iedereen die eraan werkt.
about-licences = Licenties
about-licences-hint = De volledige licentieteksten van de lettertypen, woordenboeken, bibliotheken en onderdelen van het installatieprogramma die ReasonCraft bevat. Open er een om hem te lezen.
about-licences-ubuntu = Lettertype Ubuntu Light: Ubuntu Font Licence 1.0
about-licences-hack = Lettertype Hack: MIT en de Bitstream Vera-licentie
about-licences-emoji = Lettertype egui-icons (uit emoji-icon-font): MIT
about-licences-dict_en = Engels woordenboek (SCOWL): MIT en BSD
about-licences-dict_nl = Nederlands woordenboek (OpenTaal): BSD-3-Clause
about-licences-spellbook = spellbook: Mozilla Public License 2.0
about-licences-nsis = Windows-installatieprogramma (NSIS): zlib/libpng-licentie
about-licences-crates = Rust-crates ({ $count } licentieteksten)

## Accessibility: names screen readers announce
a11y-in_group = { $group }: { $name }
a11y-resize_building = Breedte van Bouwstenen aanpassen
a11y-resize_text = Breedte van het Tekstpaneel aanpassen

## Start-up: shown when the window cannot open
startup-failed-title = ReasonCraft kon niet starten
startup-failed-body = ReasonCraft kon zijn venster niet openen. Het grafische stuurprogramma van deze computer ontbreekt misschien of is verouderd; bijwerken helpt meestal.
startup-failed-details = Technische details: { $details }
