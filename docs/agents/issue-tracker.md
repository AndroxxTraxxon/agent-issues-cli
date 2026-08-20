# Issue tracker: Local SQLite

Issues for this repository are stored in the SQLite database at `.scratch/issues.db`.

Only touch the database through the `issues` CLI.

## When a skill says "publish to the issue tracker"

Create an issue:

    issues create --title "<title>" --body-file "<body-path>"

## When a skill says "fetch the relevant ticket"

Read an issue:

    issues get <id>

## When a skill says "apply the <role> triage label"

    issues label <id> add <role>

The role is one of the triage labels below.

## Commands

List issues:

    issues list
    issues list --label ready-for-agent
    issues list --label needs-triage

Update an issue (status, title, or body):

    issues update <id> --status <status>
    issues update <id> --title "<title>"
    issues update <id> --body "<body>"
    issues update <id> --body-file "<path>"
    issues update <id> --append-body "<text>"
    issues update <id> --append-body-file "<path>"

Add/remove labels:

    issues label <id> add <label>
    issues label <id> remove <label>

Close an issue (optionally with a resolution comment):

    issues close <id>
    issues close <id> --comment "<resolution>"

Add a comment:

    issues comment <id> --body "<text>"
    issues comment <id> --body-file "<path>"

## Commit anchors

Every issue records two commit anchors: the commit it was opened at and the commit that resolved it. Both come from git HEAD automatically when the working directory is a git repository, and both can be overridden or set by hand:

    issues create --title "<title>" --opened-at <sha>
    issues close <id> --resolved-by <sha>
    issues update <id> --opened-at <sha>
    issues update <id> --resolved-by <sha>
    issues update <id> --clear-opened-at
    issues update <id> --clear-resolved-by

Outside a git repository (or without git installed) no anchor is recorded; the tracker does not fail. A hash that is absent from the repository warns but is still recorded.

Add/remove a blocking dependency:

    issues depends add <issue-id> --on <dependency-id>
    issues depends remove <issue-id> --on <dependency-id>

Attach/detach an issue to a parent (membership, not a blocker):

    issues attach <issue-id> --parent <map-id>
    issues detach <issue-id> --parent <map-id>

## Wayfinding operations

Used by `/wayfinder`.

- **Map**: an issue labelled `wayfinder:map`. The map body holds the Destination / Notes / Decisions-so-far / Not yet specified / Out of scope sections.
- **Child ticket**: an issue labelled `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`) attached to the map issue with `issues attach <id> --parent <map-id>`. Membership is not a blocker: the map staying open never blocks its children.
- **Blocking**: an edge added with `issues depends add <issue-id> --on <dependency-id>`. A ticket is unblocked when every issue it depends on is closed.
- **Frontier**: `issues frontier` (optionally `--label wayfinder:research` or `--map <map-id>`). Open, unblocked issues ordered by id ascending. Issues that are themselves maps (they have children) never appear. Bare `frontier` also drops any issue attached to a map; `--map <map-id>` restricts to that map's children.
- **Blocked**: `issues blocked` lists open issues and which dependency is blocking each.
- **Claim**: `issues update <id> --status in-progress` before any work.
- **Resolve**: append the answer as a comment with `issues comment <id> --body "<answer>"`, then close with `issues close <id> --comment "<resolution>"`, and append a context pointer to the map's Decisions-so-far with `issues update <map-id> --append-body-file "<pointer-path>"`.

## Agent grab queue

Open issues labelled `ready-for-agent` are fully specified and ready for an AFK agent:

    issues ready

## Human view

`--pretty`/`--human` is for people at a terminal. Use the plain output, which is byte-stable.

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
