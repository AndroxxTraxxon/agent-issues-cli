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
        .arg("2")
        .arg("--on")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("now depends on #1"));
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
        .arg("2")
        .arg("--on")
        .arg("1")
        .assert()
        .success();
    cmd(Path::new(db))
        .arg("depends")
        .arg("4")
        .arg("--on")
        .arg("2")
        .assert()
        .success();
    cmd(Path::new(db))
        .arg("depends")
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
}
