# Argdown

**Argdown** is a plain-text way of writing down arguments, used by several argument-mapping tools. ReasonCraft can save a map as Argdown and read Argdown files, so you can move maps between tools or write a map as text.

## Saving a map as Argdown

Export > **Argdown** saves the whole map, including the items under Not connected. The file holds everything about the structure and the ratings, so reading it back gives the same map. Images are left out; the export says so when the map has any. The box widths, the layout direction, the snippets and the essay are not saved in it either.

## How a map looks in Argdown

- Statements are written as `[Title]: text`. ReasonCraft makes each title from the first words of the box.
- Each card becomes an argument, titled `R1`, `R2`, … for reasons and `O1`, `O2`, … for objections, with its premises numbered `(1)`, `(2)` above a line `----` and its conclusion below it.
- `<+` means "supports" and `<-` "opposes"; `<_` is an objection aimed at a whole card.
- Ratings, source types, notes, links and comments are written as data between `{` and `}`.

## Reading an Argdown file

File > **Import Argdown…** reads an Argdown file into a new tab and lays it out. The file itself is never changed: use Save As to keep the map as a ReasonCraft file.

A file ReasonCraft wrote reads back as the same map. Files from elsewhere may use parts of Argdown that a map cannot hold, such as contradictions, tags, inference rules or one statement used in several places. ReasonCraft changes or leaves out those parts and shows a report that lists each case with its line number.

- The statement most others point at becomes the claim; other separate trees go to Not connected.
- Files can be up to 2 MB, with up to 5000 statements.

## Read on

- [Exports and printing](exports-and-print)
- [Files and recovery](files-and-recovery)
