use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params, params_from_iter};

pub const STATUSES: [&str; 4] = ["open", "in-progress", "blocked", "closed"];

#[derive(Debug)]
pub struct Issue {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug)]
pub struct IssueSummary {
    pub id: i64,
    pub title: String,
    pub status: String,
    pub labels: Vec<String>,
}

#[derive(Debug)]
pub struct Comment {
    pub id: i64,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug)]
pub struct IssueDetail {
    pub issue: Issue,
    pub labels: Vec<String>,
    pub comments: Vec<Comment>,
    pub blocked_by: Vec<i64>,
    pub blocks: Vec<i64>,
}

#[derive(Debug)]
pub struct BlockedSummary {
    pub id: i64,
    pub title: String,
    pub blocked_by: Vec<Dependency>,
}

#[derive(Debug)]
pub struct Dependency {
    pub id: i64,
    pub title: String,
    pub status: String,
}

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent directory {}", parent.display()))?;
    }
    let conn = Connection::open(path)
        .with_context(|| format!("opening database at {}", path.display()))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS issues (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            body TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'open',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS labels (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS issue_labels (
            issue_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
            label_id INTEGER NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
            PRIMARY KEY (issue_id, label_id)
        );

        CREATE TABLE IF NOT EXISTS comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            issue_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
            body TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS dependencies (
            issue_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
            depends_on INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
            PRIMARY KEY (issue_id, depends_on)
        );

        CREATE INDEX IF NOT EXISTS idx_issue_labels_issue ON issue_labels(issue_id);
        CREATE INDEX IF NOT EXISTS idx_comments_issue ON comments(issue_id);
        CREATE INDEX IF NOT EXISTS idx_dependencies_issue ON dependencies(issue_id);
        CREATE INDEX IF NOT EXISTS idx_dependencies_depends ON dependencies(depends_on);
        "#,
    )
    .context("initializing schema")?;
    Ok(())
}

pub fn create_issue(conn: &Connection, title: &str, body: &str) -> Result<Issue> {
    let title = title.trim();
    if title.is_empty() {
        bail!("title cannot be empty");
    }
    let now = now_rfc3339();
    conn.execute(
        "INSERT INTO issues (title, body, status, created_at, updated_at) VALUES (?1, ?2, 'open', ?3, ?3)",
        params![title, body, now],
    )?;
    let id = conn.last_insert_rowid();
    Ok(fetch_issue(conn, id)?.expect("issue was just inserted"))
}

pub fn list_issues(
    conn: &Connection,
    status: Option<&str>,
    label: Option<&str>,
) -> Result<Vec<IssueSummary>> {
    let mut sql = String::from("SELECT i.id, i.title, i.status FROM issues i");
    let mut conds: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();

    if label.is_some() {
        sql.push_str(" JOIN issue_labels il ON il.issue_id = i.id");
        sql.push_str(" JOIN labels lb ON lb.id = il.label_id");
    }
    if let Some(s) = status {
        conds.push(format!("i.status = ?{}", conds.len() + 1));
        args.push(s.to_string());
    }
    if let Some(l) = label {
        conds.push(format!("lb.name = ?{}", conds.len() + 1));
        args.push(l.to_string());
    }
    if !conds.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conds.join(" AND "));
    }
    sql.push_str(" ORDER BY i.id");

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params_from_iter(args))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let title: String = row.get(1)?;
        let status: String = row.get(2)?;
        let labels = labels_for(conn, id)?;
        out.push(IssueSummary {
            id,
            title,
            status,
            labels,
        });
    }
    Ok(out)
}

pub fn frontier(conn: &Connection, label: Option<&str>) -> Result<Vec<IssueSummary>> {
    open_issues(conn, label, false)
}

pub fn blocked(conn: &Connection, label: Option<&str>) -> Result<Vec<BlockedSummary>> {
    let issues = open_issues(conn, label, true)?;
    let mut out = Vec::new();
    for summary in issues {
        let blocked_by = unresolved_dependencies(conn, summary.id)?;
        out.push(BlockedSummary {
            id: summary.id,
            title: summary.title,
            blocked_by,
        });
    }
    Ok(out)
}

pub fn ready_for_agent(conn: &Connection) -> Result<Vec<IssueSummary>> {
    let mut issues = list_issues(conn, None, Some("ready-for-agent"))?;
    issues.retain(|i| i.status == "open");
    Ok(issues)
}

fn open_issues(conn: &Connection, label: Option<&str>, blocked: bool) -> Result<Vec<IssueSummary>> {
    let mut sql = format!(
        "SELECT i.id, i.title, i.status FROM issues i
         WHERE i.status = 'open'
           AND {} (SELECT 1 FROM dependencies d
                   JOIN issues dep ON dep.id = d.depends_on
                   WHERE d.issue_id = i.id AND dep.status <> 'closed')",
        if blocked { "EXISTS" } else { "NOT EXISTS" },
    );
    let mut args: Vec<String> = Vec::new();

    if let Some(l) = label {
        sql.push_str(" AND EXISTS (SELECT 1 FROM issue_labels il JOIN labels lb ON lb.id = il.label_id WHERE il.issue_id = i.id AND lb.name = ?1)");
        args.push(l.to_string());
    }
    sql.push_str(" ORDER BY i.id");

    let mut stmt = conn.prepare(&sql)?;
    let mut out = Vec::new();
    if args.is_empty() {
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            out.push(summary_from_row(conn, row)?);
        }
    } else {
        let mut rows = stmt.query(params![args[0]])?;
        while let Some(row) = rows.next()? {
            out.push(summary_from_row(conn, row)?);
        }
    }
    Ok(out)
}

fn summary_from_row(conn: &Connection, row: &Row) -> Result<IssueSummary> {
    let id: i64 = row.get(0)?;
    let title: String = row.get(1)?;
    let status: String = row.get(2)?;
    let labels = labels_for(conn, id)?;
    Ok(IssueSummary {
        id,
        title,
        status,
        labels,
    })
}

fn unresolved_dependencies(conn: &Connection, issue_id: i64) -> Result<Vec<Dependency>> {
    let mut stmt = conn.prepare(
        "SELECT dep.id, dep.title, dep.status
         FROM dependencies d
         JOIN issues dep ON dep.id = d.depends_on
         WHERE d.issue_id = ?1 AND dep.status <> 'closed'
         ORDER BY dep.id",
    )?;
    let deps = stmt.query_map(params![issue_id], |row| {
        Ok(Dependency {
            id: row.get(0)?,
            title: row.get(1)?,
            status: row.get(2)?,
        })
    })?;
    let mut out = Vec::new();
    for dep in deps {
        out.push(dep?);
    }
    Ok(out)
}

pub fn get_issue(conn: &Connection, id: i64) -> Result<Option<IssueDetail>> {
    let Some(issue) = fetch_issue(conn, id)? else {
        return Ok(None);
    };
    let labels = labels_for(conn, id)?;
    let comments = fetch_comments(conn, id)?;
    let blocked_by = dependency_ids(
        conn,
        "SELECT depends_on FROM dependencies WHERE issue_id = ?1",
        id,
    )?;
    let blocks = dependency_ids(
        conn,
        "SELECT issue_id FROM dependencies WHERE depends_on = ?1",
        id,
    )?;
    Ok(Some(IssueDetail {
        issue,
        labels,
        comments,
        blocked_by,
        blocks,
    }))
}

pub fn update_status(conn: &Connection, id: i64, status: &str) -> Result<Issue> {
    if !STATUSES.contains(&status) {
        bail!(
            "unknown status '{status}'; must be one of: {}",
            STATUSES.join(", ")
        );
    }
    let changed = conn.execute(
        "UPDATE issues SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, now_rfc3339(), id],
    )?;
    if changed == 0 {
        bail!("issue {id} not found");
    }
    Ok(fetch_issue(conn, id)?.expect("issue was just updated"))
}

pub fn close_issue(conn: &Connection, id: i64) -> Result<Issue> {
    update_status(conn, id, "closed")
}

pub fn add_label(conn: &Connection, issue_id: i64, name: &str) -> Result<bool> {
    ensure_issue_exists(conn, issue_id)?;
    let name = name.trim();
    if name.is_empty() {
        bail!("label name cannot be empty");
    }
    conn.execute(
        "INSERT OR IGNORE INTO labels (name) VALUES (?1)",
        params![name],
    )?;
    let added = conn.execute(
        "INSERT OR IGNORE INTO issue_labels (issue_id, label_id)
         SELECT ?1, id FROM labels WHERE name = ?2",
        params![issue_id, name],
    )?;
    Ok(added > 0)
}

pub fn remove_label(conn: &Connection, issue_id: i64, name: &str) -> Result<bool> {
    ensure_issue_exists(conn, issue_id)?;
    let removed = conn.execute(
        "DELETE FROM issue_labels
         WHERE issue_id = ?1
           AND label_id IN (SELECT id FROM labels WHERE name = ?2)",
        params![issue_id, name],
    )?;
    Ok(removed > 0)
}

pub fn add_comment(conn: &Connection, issue_id: i64, body: &str) -> Result<Comment> {
    ensure_issue_exists(conn, issue_id)?;
    let now = now_rfc3339();
    conn.execute(
        "INSERT INTO comments (issue_id, body, created_at) VALUES (?1, ?2, ?3)",
        params![issue_id, body, now],
    )?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "UPDATE issues SET updated_at = ?1 WHERE id = ?2",
        params![now, issue_id],
    )?;
    Ok(Comment {
        id,
        body: body.to_string(),
        created_at: now,
    })
}

pub fn add_dependency(conn: &Connection, issue_id: i64, depends_on: i64) -> Result<()> {
    ensure_issue_exists(conn, issue_id)?;
    ensure_issue_exists(conn, depends_on)?;
    if issue_id == depends_on {
        bail!("an issue cannot depend on itself");
    }
    conn.execute(
        "INSERT OR IGNORE INTO dependencies (issue_id, depends_on) VALUES (?1, ?2)",
        params![issue_id, depends_on],
    )?;
    Ok(())
}

fn fetch_issue(conn: &Connection, id: i64) -> Result<Option<Issue>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, body, status, created_at, updated_at FROM issues WHERE id = ?1",
    )?;
    let issue = stmt
        .query_row(params![id], |row| {
            Ok(Issue {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .optional()?;
    Ok(issue)
}

fn labels_for(conn: &Connection, issue_id: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT l.name FROM labels l
         JOIN issue_labels il ON il.label_id = l.id
         WHERE il.issue_id = ?1
         ORDER BY l.name",
    )?;
    let names = stmt.query_map(params![issue_id], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for name in names {
        out.push(name?);
    }
    Ok(out)
}

fn fetch_comments(conn: &Connection, issue_id: i64) -> Result<Vec<Comment>> {
    let mut stmt =
        conn.prepare("SELECT id, body, created_at FROM comments WHERE issue_id = ?1 ORDER BY id")?;
    let comments = stmt.query_map(params![issue_id], |row| {
        Ok(Comment {
            id: row.get(0)?,
            body: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;
    let mut out = Vec::new();
    for comment in comments {
        out.push(comment?);
    }
    Ok(out)
}

fn dependency_ids(conn: &Connection, sql: &str, id: i64) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(sql)?;
    let ids = stmt.query_map(params![id], |row| row.get::<_, i64>(0))?;
    let mut out = Vec::new();
    for dep in ids {
        out.push(dep?);
    }
    Ok(out)
}

fn ensure_issue_exists(conn: &Connection, id: i64) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
        params![id],
        |row| row.get(0),
    )?;
    if !exists {
        bail!("issue {id} not found");
    }
    Ok(())
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}
