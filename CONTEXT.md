# Issue Tracker Context

A local issue tracker: a CLI over a SQLite store that agents and humans use to plan work in a repository. It understands issues, statuses, labels, comments, and two kinds of edges between issues.

## Language

**Issue**:
A tracked unit of work. Every issue has a title, a body, a status, labels, and comments.
_Avoid_: Ticket (as a generic synonym), task

**Map**:
An issue that is the umbrella for an effort, identified by the `wayfinder:map` label. Its body holds the destination, notes, decisions so far, and out-of-scope sections.
_Avoid_: Epic, parent issue

**Ticket**:
An issue that belongs to a map and carries a `wayfinder:<type>` label.
_Avoid_: Subtask, item

**Membership edge**:
The parent-child link saying a ticket belongs to a map. Added with `issues attach`, removed with `issues detach`. Not a blocker; it never gates the child.
_Avoid_: Depends-on, belongs-to as a blocker

**Blocking edge**:
The dependency saying an issue cannot be worked until another issue closes. Added with `issues depends add`, removed with `issues depends remove`.
_Avoid_: Relates-to, requires

**Frontier**:
The takeable work: open issues with no open blocking edges that are not themselves parents. Scoped to one map with `issues frontier --map <id>`.
_Avoid_: Backlog, todo list

**Claim**:
Marking a ticket in-progress before working it, so concurrent sessions skip it.
_Avoid_: Assign, take

**Resolution**:
A ticket's answer, recorded as a comment on the ticket before it closes. Research tickets resolve via subagents; decision tickets resolve through a live exchange with a human.
_Avoid_: Verdict, conclusion

**Context pointer**:
A one-line gist plus link, appended to a map's Decisions-so-far when a ticket closes.
_Avoid_: Changelog entry, summary

**Human view**:
The `--pretty` (aliased `--human`) rendering of the tracker: issue bodies and comments styled as markdown with bold, emphasis, headers, and single-color code blocks; statuses and labels color-coded. For people reading at a terminal.
_Avoid_: Pretty print, colored output

**Agent view**:
The default plain output, byte-stable, that agents parse. The human view must never change it.
_Avoid_: Plain text, raw output