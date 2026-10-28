# Argdown in ReasonCraft

ReasonCraft saves a map as [Argdown](https://argdown.org) text (Export → **Argdown**) and reads Argdown files back (File → **Import Argdown…**). This page describes the part of Argdown that ReasonCraft uses and how a map is written as Argdown.

The export is meant as an interchange file: it holds everything the map's structure and evaluation hold, so reading it back gives the same map. That is why it always includes the parking area, unlike the image exports. Other Argdown tools can open it too; they show the arguments and relations, and ReasonCraft's own details as data.

## The Argdown syntax ReasonCraft reads and writes

- **Frontmatter**: YAML between two `===` lines at the very start of the file.
- **Headings**: lines starting with `#`.
- **Statements**: `[Title]: text` defines a statement; `[Title]` alone refers to it.
- **Arguments**: `<Title>: text` defines an argument; `<Title>` alone refers to it.
- **Mentions**: `@[Title]` and `@<Title>` inside a text mention a statement or an argument.
- **Relations**: an indented line under an element relates it to the element above it.
  - `+` supports, `-` attacks, `_` undercuts (attacks the inference of an argument), `><` contradicts.
  - The direction comes first or last: `<+ X` means X supports the element above (the default direction, child to parent); `+> X` means the element above supports X. The same goes for `<-`/`->` and `<_`/`_>`.
  - A line that is indented further hangs under the line above it. One tab counts as 4 spaces.
- **Premise-conclusion structures** (PCS), under an argument title:
  - numbered statements `(1) text` or `(1) [Title]: text`;
  - an inference line of 4 or more hyphens (`----`), or `--` … `--` with an inference rule in between;
  - the last statement after the inference line is the conclusion.
  - Relations can hang under a PCS statement and under the inference line.
- **Comments**: `// …` to the end of the line, `/* … */` and `<!-- … -->`.
- **Data**: `{key: value, …}` after an element, in YAML flow style: strings, numbers, `true`/`false`, lists `[…]` and maps `{…}`.
- **Escapes**: a backslash before a character makes it plain text, for example `\[` or `\#`.
- Empty lines separate top-level blocks.

## How a map is written

### 1. Frontmatter

```
===
title: "…"
author: "…"
language: "en"
generator: "ReasonCraft"
===
```

All four values are YAML double-quoted strings (see 5). An `=` in a value is written as `\u003D`, so a title such as `A === B` never ends the frontmatter. `language` is `en` or `nl`; a Dutch tag such as `nl-NL` is written as `nl`.

### 2. Titles

- Every box written as a statement gets a title made from the first five words of its text.
  - The characters `[ ] < > { } : # @` and the backslash are left out, and so is a `/` that would start a comment (`//`, `/*`) once they are gone (`x/#/y` becomes `x/y`), and so are hyphens at the start (Argdown does not read `[-…]` as a statement reference).
  - Whitespace is collapsed, and the title is cut at 40 characters.
  - A title that is already taken gets " 2", " 3", … appended. A box without text gets `Statement N`, where N counts the statements in the order of the Outline: the main tree first, then the parking area.
- Every card becomes an argument titled `R<n>` (a reason) or `O<n>` (an objection). Reasons and objections are counted separately, in the order of the Outline: the main tree first, then the parking area.

### 3. The main tree

The root is written first as `[Title]: text`. Under it, indented 4 spaces, is one line per item under the root, in the map's order:

- `<+ <Rn>` for a reason and `<- <On>` for an objection;
- `<+ [Title]: citation {source: …}` for a source (see 5);
- `<+ [Title]: text {category: true}` for a category (see 6).

### 4. Cards: arguments

After the root come the arguments, one block per card in Outline order, separated by empty lines:

```
<R1> {kind: reason, strength: strong}

(1) [Premise title]: premise text {acceptance: accept}
    <- <O2>
    <+ [Source title]: citation {source: publication, rating: solid}
(2) [Co-premise title]: co-premise text
----
    <_ <O3>
    <+ <R4>
(3) [Target title]
```

- **Header**: the argument title with data: `kind: reason` or `kind: objection` always, then `strength: strong|weak|nil` when the card is rated.
- **Premises**: one numbered statement per premise, with its data. Under each premise, indented, one line per card that targets that premise (`<+ <Rm>` or `<- <Om>`) and one line per source.
- **Inference line** `----`. Under it, one line per card that targets the whole card: `<_ <Om>` for an objection (an undercut), and `<+ <Rm>` for a reason. The second form is a ReasonCraft convention: Argdown has no relation for supporting an inference, so other Argdown tools show it as plain support of the argument.
- **Conclusion**:
  - A reason concludes its target: `(3) [Target title]`.
  - An objection concludes `(3) Not: Target title`, followed by an indented `-> [Target title]` (the conclusion attacks the target).
  - A card that targets a whole card concludes `(n) The inference in @<Rk> holds.` (a reason) or `(n) The inference in @<Rk> does not hold.` (an objection), a mention of the target argument, without a relation line; the relation is the line under that card's inference line.
  - A parked card, which targets nothing, concludes with the placeholder `(n) No target. {target: none}`. Argdown needs a conclusion after an inference line; a reader drops a conclusion marked `target: none`.
- **Reading the kind back**: the `kind` data decides whether a card is a reason or an objection. Without it, the conclusion's shape decides (`Not: …` with `->`, or `… does not hold.`, is an objection), and only then the title's `R`/`O` prefix.

### 5. Statement data, sources and notes

Data goes after the statement's text, as one `{…}` with these keys, in this order:

| Key | On | Value |
|---|---|---|
| `category` | categories | `true` |
| `source` | sources | the source type: `source`, `my_basis`, `assertion`, `by_definition`, `case_study`, `common_belief`, `data`, `event`, `example`, `expert_opinion`, `law`, `media`, `personal_experience`, `publication`, `quote`, `statistic` or `web`. A type from a newer ReasonCraft is written as it is. |
| `note` | a parked note | `true` |
| `acceptance` | claims and premises | `accept`, `unsure` or `reject` |
| `rating` | sources | `solid`, `shaky` or `nil` |
| `link` | any box | the link, as a quoted string |
| `notes` | any box with notes under it | a list of quoted strings, the notes' texts |
| `comments` | any box with comments | a list of `{author: "…", text: "…", verdict: positive\|needs_work\|problem}`; `verdict` is left out when the comment has none |

- A key without a value is left out, and a statement without any data has no `{…}`.
- Sources are statements on a relation line `<+ [Title]: citation {source: …}` under the box they belong to: the root, a premise, or (in the parking area) a parked claim.
- Notes are not statements: their texts are the `notes` data of the box they sit under. Only a parked note, which has no box above it, is a statement of its own, marked `{note: true}`.
- The root's sources, notes, comments, link and acceptance are written the same way as a premise's.
- Quoted strings use YAML double quotes: `\"`, `\\`, `\n` (line break), `\r`, `\t` and `\uXXXX` are escapes, so data keeps line breaks. Other control characters, and the braces `{` and `}`, are written as `\uXXXX` (`\u007B`, `\u007D`): Argdown ends a data block at a `}` even inside quotes.

### 6. Grouping maps

A grouping map is the root followed by its categories as a nested tree of relation lines, 4 more spaces per level:

```
[Ways to get around town]: Ways to get around town
    <+ [Public transport]: Public transport {category: true, notes: ["Check the timetables."]}
        <+ [Buses]: Buses {category: true}
    <+ [Cycling]: Cycling {category: true}
```

The categories' notes and comments are data on their lines.

### 7. The parking area

The parking area follows the main tree's arguments under the heading `# Not connected` (`# Niet verbonden` in a Dutch map), and only when it holds anything. Each parked item follows as its own block, in the parking area's order:

- a parked card is an argument block whose conclusion is the placeholder `No target. {target: none}`, followed by the argument blocks of the cards under it;
- a parked box is a statement block: the statement with its data and the relation lines under it (cards, sources, categories), followed by the argument blocks of its cards.

### 8. Text

- Statement text is written on one line: line breaks in a box's text become spaces.
- Spaces at the start and end of a box's text are not kept: Argdown trims them.
- A backslash goes before each character that would otherwise mean something in Argdown: `\ [ ] < > { } # @ * _`, and a `/` that would start a comment. The text reads back exactly, and other Argdown tools show it without the backslashes.
- Not exported: images (the export says "Images are not included in Argdown." when the map has any), box widths, comment dates, the links and comments of notes (a parked note keeps them), the layout direction, snippets and the essay.

## Reading Argdown

File → **Import Argdown…** reads any Argdown file, not just ReasonCraft's own, into a new, unsaved tab. A file ReasonCraft wrote reads back as the same map. For other files:

- **The root** is the statement with the most relations pointing at it (directly, or as the conclusion of an argument), among the top-level statements and the statements that support or attack nothing; on a tie, the first one. Premises, statements marked as a source, category or note, and statements after a `# Not connected` (or `# Niet verbonden`) heading do not count.
- **The parking area** gets the items under that heading first, in file order. Then whatever is still not placed: first the statements and arguments that support or attack nothing (whole trees), then the rest, each group in file order.
- **Statements under statements**: `+ [X]` or `<+ [X]` under a statement makes a reason card with X as its premise, `-` an objection. With `{source: …}` X becomes a source of that type (an unknown type is kept by name), with `{category: true}` a category (where a category may go).
- **Arguments** become cards. Their premises are the PCS statements, except conclusions; an argument without a PCS gets one premise with its text, or its title. An argument concluding a statement supports it; a conclusion with `-> [S]` (or `- [S]`) under it attacks S.
- **Whole cards**: `_` under an argument or its inference line makes an objection to the whole card, `+` under the inference line a reason for it.
- **The kind of a card** comes from its `kind` data, else the shape of its conclusion, else an `R1`/`O1` title, else the relation it hangs on.
- **Text**: escapes are read, inline formatting is removed, `[text](url)` keeps its text and the first link becomes the box's link (if it is a safe link), and mentions `@[T]` and `@<T>` become `T`.
- **Data**: the keys of section 5 and `kind` and `strength` are read; other keys are ignored. The frontmatter gives the title, author and language (`nl` for `nl` or a Dutch tag such as `nl-NL`, in any case; otherwise `en`). Without a title, the file's name is used.
- **Reported changes**, each with its line: a contradiction (`><`, left out), another wording of a statement already defined or another description or PCS of an argument (the first is used), a statement used in a second place (copied there), a relation that would make a circle (left out), a reference to a title defined nowhere (the title becomes the text), an intermediate conclusion (left out), an inference rule (left out), tags (left out), an undercut of a statement (read as an objection), an argument with no premises, text or title (left out), an item that cannot go where the file puts it, such as a reason under a category (moved to the parking area), a relation to a conclusion that was left out (left out; its statement goes to the parking area), a link that is not a safe web or mail link (left out) and a text longer than the map keeps (cut: a box keeps 50 000 characters, a comment 50 000, a title or author 1 000; a link longer than 8 192 characters is left out). The report groups them by kind.
- **An inference rule over several lines** (`--`, the rule, `--`) also ends at a blank line or at the next statement, argument, relation or heading, so a stray `--` hides nothing.
- **Limits**: files up to 2 MB, maps up to 5000 boxes and relations nested up to 64 levels. Titles can chain statements deeper than that; what lies more than 512 levels deep (as deep as a map goes) goes to the parking area, with a note in the report.

## Example: a map written as Argdown

A claim with a rated reason (two premises and a source), an undercut of that reason, an objection with a rebuttal and a comment, a parked reason and a parked note:

```argdown
===
title: "Car-free centre"
author: "Sam"
language: "en"
generator: "ReasonCraft"
===

[The town centre should be]: The town centre should be car-free. {acceptance: accept}
    <+ <R1>
    <- <O2>

<R1> {kind: reason, strength: strong}

(1) [Cars make the centre noisy]: Cars make the centre noisy and unsafe.
    <+ [Traffic report 2024]: Traffic report 2024 {source: statistic, rating: solid}
(2) [A quiet centre draws more]: A quiet centre draws more visitors.
----
    <_ <O1>
(3) [The town centre should be]

<O1> {kind: objection}

(1) [The report covers only one]: The report covers only one street.
----
(2) The inference in @<R1> does not hold.

<O2> {kind: objection}

(1) [Shops need delivery vans.]: Shops need delivery vans. {comments: [{author: "Sam", text: "Ask the shop owners.", verdict: needs_work}]}
    <- <O3>
----
(2) Not: The town centre should be
    -> [The town centre should be]

<O3> {kind: objection}

(1) [Deliveries can be made before]: Deliveries can be made before 10 am.
----
(2) Not: Shops need delivery vans.
    -> [Shops need delivery vans.]

# Not connected

<R2> {kind: reason}

(1) [Buses could run more often.]: Buses could run more often.
----
(2) No target. {target: none}

[Check the bus routes.]: Check the bus routes. {note: true}
```

## Example: an undercut written by hand

Argdown written elsewhere often looks like this. The argument `<Small sample>` undercuts `<Sample>`: it attacks the step from the survey to the claim, not the survey itself. ReasonCraft reads it as a reason card with the survey as its premise, and a whole-card objection on that card.

```argdown
[Car-free centre]: The town centre should be car-free.

<Sample>: Residents want it.

(1) [Survey]: Most residents in a survey want a car-free centre.
----
    <_ <Small sample>
(2) [Car-free centre]

<Small sample>: Only 40 of 12 000 residents answered the survey.
```
