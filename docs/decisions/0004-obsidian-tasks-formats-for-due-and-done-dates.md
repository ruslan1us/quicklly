# 0004. Obsidian Tasks formats for due and done dates

- **Status:** Accepted
- **Date:** 2026-10-13

## Context

Reminders need a due date per note, and the archive needs the day each note was completed. With
Markdown files as the only storage ([ADR 0002](0002-plain-markdown-files-as-the-only-storage.md)),
both must be written into the note text. Users display dates in different formats, but files must
be unambiguous and readable by other tools. Obsidian, the most likely companion tool, has a widely
used convention in the Tasks plugin.

## Decision

Store a due date as `📅 YYYY-MM-DD` anywhere in the note and the completion date as
`✅ YYYY-MM-DD` at the end of the first line of a done note (`- [x] HH:MM text ✅ YYYY-MM-DD`),
exactly as Obsidian Tasks writes them. Files always contain ISO dates. Windows show and accept
dates in the **Date format** setting and convert at every boundary between a text field and a file.

## Consequences

- Obsidian with the Tasks plugin understands Quicklly's dates and done notes without conversion.
- Two text forms exist (stored and displayed), and every place that moves text between a field and
  a file must convert explicitly; the boundaries are listed in
  [Data format](../data-format.md#display-and-stored-forms).
- Only the first valid due date of a note counts for reminders and snoozing; only a trailing done
  date counts for the archive and the journal.
- Notes completed before this decision have no done date and are never archived automatically.
- Search matches stored text, so dates are found by their ISO form.
