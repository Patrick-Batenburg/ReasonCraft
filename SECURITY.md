# Security policy

## Reporting a vulnerability

Please report security problems in private, not in a public issue: on GitHub, open the repository's **Security** tab and choose **Report a vulnerability** ([direct link](https://github.com/Patrick-Batenburg/ReasonCraft/security/advisories/new)). This uses GitHub's private vulnerability reporting; only the maintainers see the report.

Say what is affected (the version from `reasoncraft --version`, the system and how ReasonCraft was installed), how to reproduce it (a small file that triggers it helps), and what an attacker gains. The maintainers aim to answer within two weeks. Once a fix is released, the advisory is published, crediting you unless you prefer otherwise.

## Supported versions

Only the latest release gets security fixes. ReasonCraft has no automatic updates: get the newest version from the [releases page](https://github.com/Patrick-Batenburg/ReasonCraft/releases/latest).

## Scope

ReasonCraft works offline and opens files that may come from anyone, so these are in scope:

- `.rcraft` maps (a zip archive with JSON and images): anything that crashes the app, hangs it, takes unbounded memory or disk, writes outside the places the user chose, or runs code.
- Argdown text imported with File → Import Argdown, and text or images pasted from the clipboard.
- Images inside a map or inserted into one (PNG, JPEG, WebP).
- Links in a map: a link that opens something other than what the app shows, or runs a program or a file when clicked.
- Exports (SVG, PDF, PNG, Word, Markdown, Argdown) that carry more of the map than the user exported, or that harm the program opening them.
- The installers and the release downloads: for example DLL search-order problems, files installed writable by other users, or a release that does not match its checksums and attestation.
- Autosave and recovery files, the settings file and the single-instance channel, where another local user could read or change them.

Out of scope: problems that need an attacker who already controls the user's account or machine, and bugs in third-party components that do not affect ReasonCraft (report those upstream; see [THIRD-PARTY.md](THIRD-PARTY.md)).
