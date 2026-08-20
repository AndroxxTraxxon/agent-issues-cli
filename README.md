# issues

A CLI for managing agent issues in a local SQLite store.

Issues are stored in the SQLite database at `.scratch/issues.db` (override with the `--db` flag or `ISSUES_DB` env var). The database and its parent directory are created on first use; do not manipulate the database directly.

## Installation

```bash
cargo install --path .
```

The binary is named `issues`.

## Commands

Create an issue:

    issues create --title "<title>" --body-file "<path>"

Or with an inline body instead of a file:

    issues create --title "<title>" --body "<text>"

List issues:

    issues list
    issues list --status ready-for-agent
    issues list --label needs-triage

Read an issue:

    issues get <id>

Update status:

    issues update <id> --status <status>

Add/remove labels:

    issues label <id> add <label>
    issues label <id> remove <label>

Close an issue:

    issues close <id>

Add a comment:

    issues comment <id> --body "<text>"

Add dependency:

    issues depends <issue-id> --on <dependency-id>

`--on` is repeatable: `issues depends 5 --on 3 --on 4`.

## Human view

Add `--pretty` (or its alias `--human`) to any read command for a view styled
for people at a terminal: `issues get 1 --pretty` renders the body and comments
as markdown (bold, emphasis, headers, reverse-video inline code, single-color
code blocks), and the list-family commands color-code statuses and labels.

Colorization only happens when stdout is a terminal. Piped or redirected output
is identical to the plain form, so agent pipelines see byte-stable text either
way. Set `NO_COLOR` to force plain output on a terminal.

## Wayfinding and triage views

The agent grab queue (open issues labelled `ready-for-agent`):

    issues ready

The wayfinder frontier (open issues whose dependencies are all closed):

    issues frontier
    issues frontier --label wayfinder:research

Open issues blocked by an unresolved dependency (shows what is blocking each):

    issues blocked
    issues blocked --label wayfinder:task

## Agent instructions

Generate the `docs/agents/issue-tracker.md` document describing this tracker for
other agents (used by `/setup-matt-pocock-skills`):

    issues agent-instructions
    issues agent-instructions --output docs/agents/issue-tracker.md

## Canonical statuses

- open
- in-progress
- blocked
- closed

## Triage labels

- needs-triage
- needs-info
- ready-for-agent
- ready-for-human
- wontfix