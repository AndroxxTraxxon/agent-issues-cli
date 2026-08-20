pub const ISSUE_TRACKER_DOC: &str = r#"# Issue tracker: Local SQLite

Issues for this repository are stored in the SQLite database at `.scratch/issues.db`.

Do not manipulate the database directly. Use the `issues` CLI for all issue operations.

## When a skill says "publish to the issue tracker"

Create an issue:

    issues create --title "<title>" --body-file "<body-path>"

## When a skill says "fetch the relevant ticket"

Read an issue:

    issues get <id>

## When a skill says "apply the <role> triage label"

    issues label <id> add <role>

Roles are the canonical triage labels below; the string used is the label name itself.

## Commands

List issues:

    issues list
    issues list --status ready-for-agent
    issues list --label needs-triage

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

## Wayfinding operations

Used by `/wayfinder`.

- **Map**: an issue labelled `wayfinder:map`. The map body holds the Destination / Notes / Decisions-so-far / Not yet specified / Out of scope sections.
- **Child ticket**: an issue labelled `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`) that depends on the map issue.
- **Blocking**: an edge added with `issues depends <issue-id> --on <dependency-id>`. A ticket is unblocked when every issue it depends on is closed.
- **Frontier**: `issues frontier` (optionally `--label wayfinder:research`). Open, unblocked issues, first by id wins.
- **Blocked**: `issues blocked` lists open issues and which dependency is blocking each.
- **Claim**: `issues update <id> --status in-progress` before any work.
- **Resolve**: append the answer as a comment with `issues comment <id> --body "<answer>"`, then close with `issues close <id>`, and append a context pointer to the map's Decisions-so-far.

## Agent grab queue

Open issues labelled `ready-for-agent` are fully specified and ready for an AFK agent:

    issues ready

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
"#;
