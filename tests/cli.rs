use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd(db: &Path) -> Command {
    let mut c = Command::cargo_bin("issues").unwrap();
    c.arg("--db").arg(db);
    c
}

fn create_test_db() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("issues.db").display().to_string();
    (dir, db)
}

#[test]
fn create_and_get_issue() {
    let (_dir, db) = create_test_db();
    let body_file = _dir.path().join("body.md");
    std::fs::write(&body_file, "Some body text").unwrap();

    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("Add auth")
        .arg("--body-file")
        .arg(&body_file)
        .assert()
        .success()
        .stdout(predicate::str::contains("Created issue #1: Add auth"));

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Issue #1: Add auth"))
        .stdout(predicate::str::contains("Status: open"))
        .stdout(predicate::str::contains("Some body text"));
}

#[test]
fn create_with_inline_body() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("Inline")
        .arg("--body")
        .arg("hello")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn create_rejects_mutually_exclusive_body_flags() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("X")
        .arg("--body")
        .arg("a")
        .arg("--body-file")
        .arg("b.md")
        .assert()
        .failure()
        .stderr(predicate::str::contains("mutually exclusive"));
}

#[test]
fn create_requires_title() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db)).arg("create").assert().failure();
}

#[test]
fn list_empty_reports_no_issues() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues found."));
}

#[test]
fn list_filters_by_status_and_label() {
    let (_dir, db) = create_test_db();
    for title in ["one", "two", "three"] {
        cmd(Path::new(&db))
            .arg("create")
            .arg("--title")
            .arg(title)
            .assert()
            .success();
    }
    cmd(Path::new(&db))
        .arg("label")
        .arg("1")
        .arg("add")
        .arg("ready-for-agent")
        .assert()
        .success();
    cmd(Path::new(&db))
        .arg("update")
        .arg("3")
        .arg("--status")
        .arg("blocked")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("list")
        .arg("--status")
        .arg("blocked")
        .assert()
        .success()
        .stdout(predicate::str::contains("three"))
        .stdout(predicate::str::contains("1 issue(s)."));

    cmd(Path::new(&db))
        .arg("list")
        .arg("--label")
        .arg("ready-for-agent")
        .assert()
        .success()
        .stdout(predicate::str::contains("one"))
        .stdout(predicate::str::contains("1 issue(s)."));
}

#[test]
fn update_status_and_validate() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("update")
        .arg("1")
        .arg("--status")
        .arg("in-progress")
        .assert()
        .success()
        .stdout(predicate::str::contains("is now in-progress"));

    cmd(Path::new(&db))
        .arg("update")
        .arg("1")
        .arg("--status")
        .arg("bogus")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown status"));
}

#[test]
fn label_add_remove() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("label")
        .arg("1")
        .arg("add")
        .arg("needs-triage")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added label 'needs-triage'"));

    cmd(Path::new(&db))
        .arg("label")
        .arg("1")
        .arg("add")
        .arg("needs-triage")
        .assert()
        .success()
        .stdout(predicate::str::contains("already has label"));

    cmd(Path::new(&db))
        .arg("label")
        .arg("1")
        .arg("remove")
        .arg("needs-triage")
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed label 'needs-triage'"));

    cmd(Path::new(&db))
        .arg("label")
        .arg("1")
        .arg("remove")
        .arg("needs-triage")
        .assert()
        .success()
        .stdout(predicate::str::contains("does not have label"));
}

#[test]
fn close_issue() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("close")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Closed issue #1"));

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: closed"));
}

#[test]
fn comments_show_on_get() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("comment")
        .arg("1")
        .arg("--body")
        .arg("first note")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added comment #1"));

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("first note"));
}

#[test]
fn dependencies_render_on_get() {
    let (_dir, db) = create_test_db();
    for title in ["first", "second"] {
        cmd(Path::new(&db))
            .arg("create")
            .arg("--title")
            .arg(title)
            .assert()
            .success();
    }

    cmd(Path::new(&db))
        .arg("depends")
        .arg("add")
        .arg("2")
        .arg("--on")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Issue #2 now depends on #1"));

    cmd(Path::new(&db))
        .arg("get")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("Blocked by: #1"));

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Blocks: #2"));

    cmd(Path::new(&db))
        .arg("depends")
        .arg("add")
        .arg("2")
        .arg("--on")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("now depends on #1"));
}

#[test]
fn depends_remove_clears_edge() {
    let (_dir, db) = create_test_db();
    for title in ["first", "second"] {
        cmd(Path::new(&db))
            .arg("create")
            .arg("--title")
            .arg(title)
            .assert()
            .success();
    }

    cmd(Path::new(&db))
        .arg("depends")
        .arg("add")
        .arg("2")
        .arg("--on")
        .arg("1")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("depends")
        .arg("remove")
        .arg("2")
        .arg("--on")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("no longer depends on #1"));

    cmd(Path::new(&db))
        .arg("get")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("Blocked by: #1").not());

    cmd(Path::new(&db))
        .arg("depends")
        .arg("remove")
        .arg("2")
        .arg("--on")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("does not depend on #1"));
}

#[test]
fn depends_rejects_self_dependency() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("depends")
        .arg("add")
        .arg("1")
        .arg("--on")
        .arg("1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot depend on itself"));
}

#[test]
fn missing_issue_errors() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("get")
        .arg("42")
        .assert()
        .failure()
        .stderr(predicate::str::contains("issue 42 not found"));

    cmd(Path::new(&db))
        .arg("comment")
        .arg("42")
        .arg("--body")
        .arg("x")
        .assert()
        .failure()
        .stderr(predicate::str::contains("issue 42 not found"));
}

#[test]
fn creates_db_in_scratch_dir() {
    let dir = TempDir::new().unwrap();
    let project = dir.path().join("proj");
    std::fs::create_dir(&project).unwrap();

    Command::cargo_bin("issues")
        .unwrap()
        .current_dir(&project)
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    assert!(project.join(".scratch").join("issues.db").exists());
}

fn seed_wayfinder(db: &str) {
    for title in ["auth", "rate limit", "logging", "deploy"] {
        cmd(Path::new(db))
            .arg("create")
            .arg("--title")
            .arg(title)
            .assert()
            .success();
    }
    // 2 depends on 1; 4 depends on 2 and 3
    cmd(Path::new(db))
        .arg("depends")
        .arg("add")
        .arg("2")
        .arg("--on")
        .arg("1")
        .assert()
        .success();
    cmd(Path::new(db))
        .arg("depends")
        .arg("add")
        .arg("4")
        .arg("--on")
        .arg("2")
        .assert()
        .success();
    cmd(Path::new(db))
        .arg("depends")
        .arg("add")
        .arg("4")
        .arg("--on")
        .arg("3")
        .assert()
        .success();
}

#[test]
fn frontier_lists_open_unblocked_issues() {
    let (_dir, db) = create_test_db();
    seed_wayfinder(&db);

    cmd(Path::new(&db))
        .arg("frontier")
        .assert()
        .success()
        .stdout(predicate::str::contains("auth"))
        .stdout(predicate::str::contains("logging"))
        .stdout(predicate::str::contains("2 issue(s)."))
        .stdout(predicate::str::contains("rate limit").not())
        .stdout(predicate::str::contains("deploy").not());

    // closing the first dep makes "rate limit" takeable
    cmd(Path::new(&db)).arg("close").arg("1").assert().success();
    cmd(Path::new(&db))
        .arg("frontier")
        .assert()
        .success()
        .stdout(predicate::str::contains("rate limit"))
        .stdout(predicate::str::contains("deploy").not());
}

#[test]
fn frontier_filters_by_label() {
    let (_dir, db) = create_test_db();
    seed_wayfinder(&db);
    cmd(Path::new(&db))
        .arg("label")
        .arg("1")
        .arg("add")
        .arg("wayfinder:research")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("frontier")
        .arg("--label")
        .arg("wayfinder:research")
        .assert()
        .success()
        .stdout(predicate::str::contains("auth"))
        .stdout(predicate::str::contains("1 issue(s)."));
}

#[test]
fn blocked_lists_unresolved_dependencies() {
    let (_dir, db) = create_test_db();
    seed_wayfinder(&db);

    cmd(Path::new(&db))
        .arg("blocked")
        .assert()
        .success()
        .stdout(predicate::str::contains("rate limit"))
        .stdout(predicate::str::contains("#1 auth (open)"))
        .stdout(predicate::str::contains("#2 rate limit (open)"))
        .stdout(predicate::str::contains("2 blocked issue(s)."));
}

#[test]
fn ready_shows_ready_for_agent_only() {
    let (_dir, db) = create_test_db();
    seed_wayfinder(&db);
    cmd(Path::new(&db))
        .arg("label")
        .arg("1")
        .arg("add")
        .arg("ready-for-agent")
        .assert()
        .success();
    cmd(Path::new(&db))
        .arg("label")
        .arg("2")
        .arg("add")
        .arg("ready-for-agent")
        .assert()
        .success();
    cmd(Path::new(&db))
        .arg("update")
        .arg("2")
        .arg("--status")
        .arg("in-progress")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("ready")
        .assert()
        .success()
        .stdout(predicate::str::contains("auth"))
        .stdout(predicate::str::contains("1 issue(s)."))
        // in-progress but labelled ready should not appear in the grab queue
        .stdout(predicate::str::contains("rate limit").not());
}

#[test]
fn agent_instructions_writes_doc() {
    let dir = TempDir::new().unwrap();
    let project = dir.path().join("proj");
    std::fs::create_dir(&project).unwrap();

    Command::cargo_bin("issues")
        .unwrap()
        .current_dir(&project)
        .arg("agent-instructions")
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote agent instructions"));

    let doc = project.join("docs/agents/issue-tracker.md");
    assert!(doc.exists());
    let contents = std::fs::read_to_string(&doc).unwrap();
    assert!(contents.contains("issues list"));
    assert!(contents.contains("ready-for-agent"));
    assert!(contents.contains("issues frontier"));
    assert!(contents.contains("issues depends add"));
    assert!(contents.contains("issues attach"));
}

#[test]
fn update_title_body_and_append() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("original")
        .arg("--body")
        .arg("base body")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("update")
        .arg("1")
        .arg("--title")
        .arg("renamed")
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated issue #1: renamed"));

    cmd(Path::new(&db))
        .arg("update")
        .arg("1")
        .arg("--append-body")
        .arg(" appended bit")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("update")
        .arg("1")
        .arg("--body")
        .arg("replaced")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Issue #1: renamed"))
        .stdout(predicate::str::contains("replaced"))
        .stdout(predicate::str::contains("base body").not())
        .stdout(predicate::str::contains("appended bit").not());

    cmd(Path::new(&db))
        .arg("update")
        .arg("1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("nothing to update"));
}

#[test]
fn update_body_from_file() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    let body_file = _dir.path().join("new_body.md");
    std::fs::write(&body_file, "from file").unwrap();
    cmd(Path::new(&db))
        .arg("update")
        .arg("1")
        .arg("--body-file")
        .arg(&body_file)
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("from file"));
}

#[test]
fn close_with_comment_records_comment() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("close")
        .arg("1")
        .arg("--comment")
        .arg("done via subagent")
        .assert()
        .success()
        .stdout(predicate::str::contains("Closed issue #1"))
        .stdout(predicate::str::contains("Added comment #1"));

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: closed"))
        .stdout(predicate::str::contains("done via subagent"));
}

#[test]
fn comment_body_from_file() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    let body_file = _dir.path().join("comment.md");
    std::fs::write(&body_file, "comment from file").unwrap();
    cmd(Path::new(&db))
        .arg("comment")
        .arg("1")
        .arg("--body-file")
        .arg(&body_file)
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("comment from file"));
}

fn seed_map(db: &str) {
    cmd(Path::new(db))
        .arg("create")
        .arg("--title")
        .arg("map")
        .assert()
        .success();
    for title in ["child a", "child b", "orphan"] {
        cmd(Path::new(db))
            .arg("create")
            .arg("--title")
            .arg(title)
            .assert()
            .success();
    }
    cmd(Path::new(db))
        .arg("attach")
        .arg("2")
        .arg("--parent")
        .arg("1")
        .assert()
        .success();
    cmd(Path::new(db))
        .arg("attach")
        .arg("3")
        .arg("--parent")
        .arg("1")
        .assert()
        .success();
}

#[test]
fn attach_detach_and_frontier_excludes_parents() {
    let (_dir, db) = create_test_db();
    seed_map(&db);

    cmd(Path::new(&db))
        .arg("get")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("Parents: #1"));
    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Children: #2, #3"));

    cmd(Path::new(&db))
        .arg("list")
        .arg("--parent")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("child a"))
        .stdout(predicate::str::contains("child b"))
        .stdout(predicate::str::contains("orphan").not());

    cmd(Path::new(&db))
        .arg("frontier")
        .assert()
        .success()
        .stdout(predicate::str::contains("orphan"))
        .stdout(predicate::str::contains("map").not())
        .stdout(predicate::str::contains("child a").not())
        .stdout(predicate::str::contains("1 issue(s)."));

    cmd(Path::new(&db))
        .arg("frontier")
        .arg("--map")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("child a"))
        .stdout(predicate::str::contains("child b"))
        .stdout(predicate::str::contains("orphan").not())
        .stdout(predicate::str::contains("2 issue(s)."));

    cmd(Path::new(&db))
        .arg("detach")
        .arg("2")
        .arg("--parent")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Detached issue #2 from #1"));

    cmd(Path::new(&db))
        .arg("get")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("Parents").not());
}

#[test]
fn attach_rejects_self_parent() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("attach")
        .arg("1")
        .arg("--parent")
        .arg("1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be its own parent"));
}

#[test]
fn attach_is_idempotent() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("parent")
        .assert()
        .success();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("child")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("attach")
        .arg("2")
        .arg("--parent")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Attached issue #2 to #1"));

    cmd(Path::new(&db))
        .arg("attach")
        .arg("2")
        .arg("--parent")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("already attached"));
}

#[test]
fn pretty_and_human_flags_are_accepted() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .arg("--pretty")
        .assert()
        .success();
    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .arg("--human")
        .assert()
        .success();
    cmd(Path::new(&db))
        .arg("list")
        .arg("--pretty")
        .assert()
        .success();
    cmd(Path::new(&db))
        .arg("frontier")
        .arg("--pretty")
        .assert()
        .success();
    cmd(Path::new(&db))
        .arg("blocked")
        .arg("--pretty")
        .assert()
        .success();
    cmd(Path::new(&db))
        .arg("ready")
        .arg("--pretty")
        .assert()
        .success();
}

#[test]
fn pretty_piped_output_matches_plain() {
    let (_dir, db) = create_test_db();
    let body = "**bold** and `code`\n\n## H2\n\n- one\n- two\n";
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .arg("--body")
        .arg(body)
        .assert()
        .success();
    cmd(Path::new(&db))
        .arg("comment")
        .arg("1")
        .arg("--body")
        .arg("note")
        .assert()
        .success();

    let plain = cmd(Path::new(&db)).arg("get").arg("1").output().unwrap();
    let pretty = cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .arg("--pretty")
        .output()
        .unwrap();
    assert_eq!(plain.status.code(), pretty.status.code());
    assert_eq!(plain.stdout, pretty.stdout);

    let list_plain = cmd(Path::new(&db)).arg("list").output().unwrap();
    let list_pretty = cmd(Path::new(&db))
        .arg("list")
        .arg("--pretty")
        .output()
        .unwrap();
    assert_eq!(list_plain.stdout, list_pretty.stdout);
}

#[test]
fn create_records_opened_at_from_flag() {
    let (_dir, db) = create_test_db();
    let sha = "a".repeat(40);
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .arg("--opened-at")
        .arg(&sha)
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("Opened at: {sha}")))
        .stdout(predicate::str::contains("Resolved by").not());
}

#[test]
fn close_records_resolved_by_from_flag() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    let sha = "b".repeat(40);
    cmd(Path::new(&db))
        .arg("close")
        .arg("1")
        .arg("--resolved-by")
        .arg(&sha)
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("Resolved by: {sha}")));
}

#[test]
fn update_sets_and_clears_anchors() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    let opened = "c".repeat(40);
    let resolved = "d".repeat(40);
    cmd(Path::new(&db))
        .arg("update")
        .arg("1")
        .arg("--opened-at")
        .arg(&opened)
        .arg("--resolved-by")
        .arg(&resolved)
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("Opened at: {opened}")))
        .stdout(predicate::str::contains(format!("Resolved by: {resolved}")));

    cmd(Path::new(&db))
        .arg("update")
        .arg("1")
        .arg("--clear-opened-at")
        .arg("--clear-resolved-by")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Opened at").not())
        .stdout(predicate::str::contains("Resolved by").not());
}

#[test]
fn update_rejects_conflicting_anchor_flags() {
    let (_dir, db) = create_test_db();
    cmd(Path::new(&db))
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    cmd(Path::new(&db))
        .arg("update")
        .arg("1")
        .arg("--opened-at")
        .arg("e")
        .arg("--clear-opened-at")
        .assert()
        .failure()
        .stderr(predicate::str::contains("mutually exclusive"));
}

#[test]
fn create_outside_git_records_no_anchor() {
    let dir = TempDir::new().unwrap();
    let project = dir.path().join("proj");
    std::fs::create_dir(&project).unwrap();
    let db = dir.path().join("issues.db").display().to_string();

    cmd(Path::new(&db))
        .current_dir(&project)
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    cmd(Path::new(&db))
        .current_dir(&project)
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Opened at").not())
        .stdout(predicate::str::contains("Resolved by").not());
}

fn git_in(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn init_git_repo(dir: &Path) -> String {
    git_in(dir, &["init", "-q"]);
    git_in(dir, &["config", "user.email", "test@example.com"]);
    git_in(dir, &["config", "user.name", "Test"]);
    std::fs::write(dir.join("file.txt"), "x").unwrap();
    git_in(dir, &["add", "file.txt"]);
    git_in(dir, &["commit", "-q", "-m", "init"]);
    let out = std::process::Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

#[test]
fn create_and_close_auto_stamp_head_in_git_repo() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let head = init_git_repo(&repo);
    let db = dir.path().join("issues.db").display().to_string();

    cmd(Path::new(&db))
        .current_dir(&repo)
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    cmd(Path::new(&db))
        .current_dir(&repo)
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("Opened at: {head}")));

    cmd(Path::new(&db))
        .current_dir(&repo)
        .arg("close")
        .arg("1")
        .assert()
        .success();

    cmd(Path::new(&db))
        .current_dir(&repo)
        .arg("get")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("Resolved by: {head}")));
}

#[test]
fn unknown_commit_warns_in_repo() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    init_git_repo(&repo);
    let db = dir.path().join("issues.db").display().to_string();

    cmd(Path::new(&db))
        .current_dir(&repo)
        .arg("create")
        .arg("--title")
        .arg("t")
        .assert()
        .success();

    let bogus = "f".repeat(40);
    cmd(Path::new(&db))
        .current_dir(&repo)
        .arg("update")
        .arg("1")
        .arg("--opened-at")
        .arg(&bogus)
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: commit"));
}
