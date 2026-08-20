use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use issues::cli::{Cli, Command, DependsAction, LabelAction};
use issues::db::{self, BlockedSummary, IssueDetail, IssueSummary};
use issues::git;
use issues::instructions;
use issues::render;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db_path = cli
        .db
        .unwrap_or_else(|| PathBuf::from(".scratch/issues.db"));
    let conn =
        db::open(&db_path).with_context(|| format!("opening database at {}", db_path.display()))?;
    let pretty = cli.pretty && render::should_colorize();

    match cli.command {
        Command::Create(args) => {
            if args.body.is_some() && args.body_file.is_some() {
                anyhow::bail!("--body and --body-file are mutually exclusive");
            }
            let body = match args.body_file {
                Some(path) => std::fs::read_to_string(&path)
                    .with_context(|| format!("reading body file {}", path.display()))?,
                None => args.body.unwrap_or_default(),
            };
            let opened_at = match &args.opened_at {
                Some(hash) => {
                    warn_if_unknown_commit(hash);
                    Some(hash.clone())
                }
                None => git::head(),
            };
            let issue = db::create_issue(&conn, &args.title, &body, opened_at.as_deref())?;
            println!("Created issue #{}: {}", issue.id, issue.title);
        }
        Command::List {
            status,
            label,
            parent,
        } => {
            let issues = db::list_issues(&conn, status.as_deref(), label.as_deref(), parent)?;
            print_list(&issues, pretty);
        }
        Command::Get { id } => match db::get_issue(&conn, id)? {
            Some(detail) => print_detail(&detail, pretty),
            None => anyhow::bail!("issue {id} not found"),
        },
        Command::Frontier { label, map } => {
            let issues = db::frontier(&conn, label.as_deref(), map)?;
            print_list(&issues, pretty);
        }
        Command::Blocked { label } => {
            let issues = db::blocked(&conn, label.as_deref())?;
            print_blocked(&issues, pretty);
        }
        Command::Ready => {
            let issues = db::ready_for_agent(&conn)?;
            print_list(&issues, pretty);
        }
        Command::AgentInstructions { output } => {
            let path = output.unwrap_or_else(|| PathBuf::from("docs/agents/issue-tracker.md"));
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating parent directory {}", parent.display()))?;
            }
            std::fs::write(&path, instructions::ISSUE_TRACKER_DOC)
                .with_context(|| format!("writing {}", path.display()))?;
            println!("Wrote agent instructions to {}", path.display());
        }
        Command::Update(args) => {
            if let Some(status) = &args.status {
                if args.title.is_some()
                    || args.body.is_some()
                    || args.body_file.is_some()
                    || args.append_body.is_some()
                    || args.append_body_file.is_some()
                    || args.opened_at.is_some()
                    || args.clear_opened_at
                    || args.resolved_by.is_some()
                    || args.clear_resolved_by
                {
                    anyhow::bail!(
                        "--status cannot be combined with title, body, or commit-anchor edits"
                    );
                }
                let issue = db::update_status(&conn, args.id, status)?;
                println!(
                    "Issue #{}: {} is now {}",
                    issue.id, issue.title, issue.status
                );
            } else {
                if args.body.is_some() && args.body_file.is_some() {
                    anyhow::bail!("--body and --body-file are mutually exclusive");
                }
                if args.append_body.is_some() && args.append_body_file.is_some() {
                    anyhow::bail!("--append-body and --append-body-file are mutually exclusive");
                }
                if (args.body.is_some() || args.body_file.is_some())
                    && (args.append_body.is_some() || args.append_body_file.is_some())
                {
                    anyhow::bail!("replace and append body options are mutually exclusive");
                }
                if args.opened_at.is_some() && args.clear_opened_at {
                    anyhow::bail!("--opened-at and --clear-opened-at are mutually exclusive");
                }
                if args.resolved_by.is_some() && args.clear_resolved_by {
                    anyhow::bail!("--resolved-by and --clear-resolved-by are mutually exclusive");
                }
                let opened_at_edit = match (&args.opened_at, args.clear_opened_at) {
                    (Some(hash), false) => {
                        warn_if_unknown_commit(hash);
                        Some(Some(hash.as_str()))
                    }
                    (None, true) => Some(None),
                    _ => None,
                };
                let resolved_by_edit = match (&args.resolved_by, args.clear_resolved_by) {
                    (Some(hash), false) => {
                        warn_if_unknown_commit(hash);
                        Some(Some(hash.as_str()))
                    }
                    (None, true) => Some(None),
                    _ => None,
                };
                let body_file = read_opt_file(args.body_file)?;
                let append_body_file = read_opt_file(args.append_body_file)?;
                let has_edit = body_file.is_some()
                    || args.body.is_some()
                    || args.title.is_some()
                    || append_body_file.is_some()
                    || args.append_body.is_some()
                    || opened_at_edit.is_some()
                    || resolved_by_edit.is_some();
                if !has_edit {
                    anyhow::bail!(
                        "nothing to update; provide --status, --title, --body, --body-file, --append-body, --append-body-file, --opened-at, --resolved-by, --clear-opened-at, or --clear-resolved-by"
                    );
                }
                let issue = if let Some(b) = &body_file {
                    db::update_issue(&conn, args.id, args.title.as_deref(), Some(b))?
                } else if args.body.is_some() || args.title.is_some() {
                    db::update_issue(&conn, args.id, args.title.as_deref(), args.body.as_deref())?
                } else if let Some(t) = &append_body_file {
                    db::append_body(&conn, args.id, t)?
                } else if let Some(t) = &args.append_body {
                    db::append_body(&conn, args.id, t)?
                } else {
                    db::get_issue(&conn, args.id)?.expect("issue exists").issue
                };
                let issue = if let Some(commit) = opened_at_edit {
                    db::set_opened_at_commit(&conn, args.id, commit)?
                } else {
                    issue
                };
                let issue = if let Some(commit) = resolved_by_edit {
                    db::set_resolved_by_commit(&conn, args.id, commit)?
                } else {
                    issue
                };
                println!("Updated issue #{}: {}", issue.id, issue.title);
            }
        }
        Command::Label { id, action } => match action {
            LabelAction::Add { name } => {
                if db::add_label(&conn, id, &name)? {
                    println!("Added label '{name}' to issue #{id}");
                } else {
                    println!("Issue #{id} already has label '{name}'");
                }
            }
            LabelAction::Remove { name } => {
                if db::remove_label(&conn, id, &name)? {
                    println!("Removed label '{name}' from issue #{id}");
                } else {
                    println!("Issue #{id} does not have label '{name}'");
                }
            }
        },
        Command::Close(args) => {
            let resolved_by = match &args.resolved_by {
                Some(hash) => {
                    warn_if_unknown_commit(hash);
                    Some(hash.clone())
                }
                None => git::head(),
            };
            let issue = db::close_issue(&conn, args.id, resolved_by.as_deref())?;
            println!("Closed issue #{}: {}", issue.id, issue.title);
            if let Some(body) = args.comment {
                let comment = db::add_comment(&conn, args.id, &body)?;
                println!("Added comment #{}", comment.id);
            }
        }
        Command::Comment(args) => {
            let body = match (&args.body, args.body_file) {
                (Some(_), Some(_)) => {
                    anyhow::bail!("--body and --body-file are mutually exclusive")
                }
                (Some(b), None) => b.clone(),
                (None, Some(path)) => std::fs::read_to_string(&path)
                    .with_context(|| format!("reading body file {}", path.display()))?,
                (None, None) => anyhow::bail!("provide --body or --body-file"),
            };
            let comment = db::add_comment(&conn, args.id, &body)?;
            println!("Added comment #{} to issue #{}", comment.id, args.id);
        }
        Command::Depends { action } => match action {
            DependsAction::Add(args) => {
                if args.on.is_empty() {
                    println!("No dependencies given (use --on <DEPENDENCY_ID>)");
                }
                for dependency_id in &args.on {
                    db::add_dependency(&conn, args.id, *dependency_id)?;
                    println!("Issue #{} now depends on #{dependency_id}", args.id);
                }
            }
            DependsAction::Remove(args) => {
                if args.on.is_empty() {
                    println!("No dependencies given (use --on <DEPENDENCY_ID>)");
                }
                for dependency_id in &args.on {
                    if db::remove_dependency(&conn, args.id, *dependency_id)? {
                        println!("Issue #{} no longer depends on #{dependency_id}", args.id);
                    } else {
                        println!("Issue #{} does not depend on #{dependency_id}", args.id);
                    }
                }
            }
        },
        Command::Attach(args) => {
            if args.parent.is_empty() {
                println!("No parents given (use --parent <PARENT_ID>)");
            }
            for parent_id in &args.parent {
                if db::add_membership(&conn, args.id, *parent_id)? {
                    println!("Attached issue #{} to #{parent_id}", args.id);
                } else {
                    println!("Issue #{} is already attached to #{parent_id}", args.id);
                }
            }
        }
        Command::Detach(args) => {
            if args.parent.is_empty() {
                println!("No parents given (use --parent <PARENT_ID>)");
            }
            for parent_id in &args.parent {
                if db::remove_membership(&conn, args.id, *parent_id)? {
                    println!("Detached issue #{} from #{parent_id}", args.id);
                } else {
                    println!("Issue #{} is not attached to #{parent_id}", args.id);
                }
            }
        }
    }

    Ok(())
}

fn print_list(issues: &[IssueSummary], pretty: bool) {
    if issues.is_empty() {
        println!("No issues found.");
        return;
    }
    let id_w = issues
        .iter()
        .map(|i| i.id.to_string().len())
        .max()
        .unwrap_or(2)
        .max(2);
    let status_w = issues
        .iter()
        .map(|i| i.status.len())
        .max()
        .unwrap_or(6)
        .max(6);
    let labels_w = issues
        .iter()
        .map(|i| label_text(&i.labels).len())
        .max()
        .unwrap_or(6)
        .max(6);

    println!(
        "{:<id_w$}  {:<status_w$}  {:<labels_w$}  TITLE",
        "ID", "STATUS", "LABELS"
    );
    for issue in issues {
        if pretty {
            let status_cell = padded(render::paint_status(&issue.status), status_w);
            let labels_cell = padded(render::paint_label(&label_text(&issue.labels)), labels_w);
            println!(
                "{:<id_w$}  {}  {}  {}",
                issue.id, status_cell, labels_cell, issue.title,
            );
        } else {
            println!(
                "{:<id_w$}  {:<status_w$}  {:<labels_w$}  {}",
                issue.id,
                issue.status,
                label_text(&issue.labels),
                issue.title,
            );
        }
    }
    println!("{} issue(s).", issues.len());
}

fn padded(styled: String, width: usize) -> String {
    let plain = plain_len(&styled);
    let mut s = styled;
    s.push_str(&" ".repeat(width.saturating_sub(plain)));
    s
}

fn plain_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_esc = false;
    for c in s.chars() {
        if in_esc {
            if c == 'm' {
                in_esc = false;
            }
        } else if c == '\x1b' {
            in_esc = true;
        } else {
            len += 1;
        }
    }
    len
}

fn print_blocked(issues: &[BlockedSummary], pretty: bool) {
    if issues.is_empty() {
        println!("No blocked issues.");
        return;
    }
    let id_w = issues
        .iter()
        .map(|i| i.id.to_string().len())
        .max()
        .unwrap_or(2)
        .max(2);
    let title_w = issues
        .iter()
        .map(|i| i.title.len())
        .max()
        .unwrap_or(5)
        .max(5);
    let blocked_w = issues
        .iter()
        .map(|i| blocked_text(i, false).len())
        .max()
        .unwrap_or(10)
        .max(10);

    println!(
        "{:<id_w$}  {:<title_w$}  {:<blocked_w$}",
        "ID", "TITLE", "BLOCKED BY"
    );
    for issue in issues {
        println!(
            "{:<id_w$}  {:<title_w$}  {}",
            issue.id,
            issue.title,
            blocked_text(issue, pretty),
        );
    }
    println!("{} blocked issue(s).", issues.len());
}

fn blocked_text(issue: &BlockedSummary, pretty: bool) -> String {
    issue
        .blocked_by
        .iter()
        .map(|d| {
            if pretty {
                format!(
                    "#{} {} ({})",
                    d.id,
                    d.title,
                    render::paint_status(&d.status)
                )
            } else {
                format!("#{} {} ({})", d.id, d.title, d.status)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn read_opt_file(path: Option<PathBuf>) -> Result<Option<String>> {
    match path {
        Some(path) => std::fs::read_to_string(&path)
            .map(Some)
            .with_context(|| format!("reading body file {}", path.display())),
        None => Ok(None),
    }
}

fn warn_if_unknown_commit(hash: &str) {
    if git::exists(hash) == Some(false) {
        eprintln!("warning: commit {hash} is not in the current git repository");
    }
}

fn label_text(labels: &[String]) -> String {
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels.join(", ")
    }
}

fn print_detail(detail: &IssueDetail, pretty: bool) {
    let issue = &detail.issue;
    if pretty {
        print_detail_pretty(detail);
        return;
    }
    println!("Issue #{}: {}", issue.id, issue.title);
    println!("Status: {}", issue.status);
    println!("Labels: {}", label_text(&detail.labels));
    println!("Created: {}", issue.created_at);
    println!("Updated: {}", issue.updated_at);
    if let Some(commit) = &issue.opened_at_commit {
        println!("Opened at: {}", commit);
    }
    if let Some(commit) = &issue.resolved_by_commit {
        println!("Resolved by: {}", commit);
    }

    let blocked_by = join_ids(&detail.blocked_by);
    let blocks = join_ids(&detail.blocks);
    let parents = join_ids(&detail.parents);
    let children = join_ids(&detail.children);
    if !blocked_by.is_empty() {
        println!("\nBlocked by: {}", blocked_by);
    }
    if !blocks.is_empty() {
        println!("Blocks: {}", blocks);
    }
    if !parents.is_empty() {
        println!("\nParents: {}", parents);
    }
    if !children.is_empty() {
        println!("Children: {}", children);
    }

    if !issue.body.trim().is_empty() {
        println!("\n{}", issue.body.trim_end());
    }

    if !detail.comments.is_empty() {
        println!("\nComments");
        println!("--------");
        for comment in &detail.comments {
            println!("#{} {}: {}", comment.id, comment.created_at, comment.body);
        }
    }
}

fn print_detail_pretty(detail: &IssueDetail) {
    let issue = &detail.issue;
    println!(
        "{}",
        render::paint_title(&format!("Issue #{}: {}", issue.id, issue.title))
    );
    println!("Status: {}", render::paint_status(&issue.status));
    println!(
        "Labels: {}",
        render::paint_label(&label_text(&detail.labels))
    );
    println!("Created: {}", issue.created_at);
    println!("Updated: {}", issue.updated_at);
    if let Some(commit) = &issue.opened_at_commit {
        println!("Opened at: {}", commit);
    }
    if let Some(commit) = &issue.resolved_by_commit {
        println!("Resolved by: {}", commit);
    }

    let blocked_by = join_ids(&detail.blocked_by);
    let blocks = join_ids(&detail.blocks);
    let parents = join_ids(&detail.parents);
    let children = join_ids(&detail.children);
    if !blocked_by.is_empty() {
        println!("\nBlocked by: {}", blocked_by);
    }
    if !blocks.is_empty() {
        println!("Blocks: {}", blocks);
    }
    if !parents.is_empty() {
        println!("\nParents: {}", parents);
    }
    if !children.is_empty() {
        println!("Children: {}", children);
    }

    if !issue.body.trim().is_empty() {
        println!();
        print!("{}", render::render_markdown(&issue.body).trim_end());
        println!();
    }

    if !detail.comments.is_empty() {
        println!();
        println!("{}", render::paint_title("Comments"));
        println!("--------");
        for comment in &detail.comments {
            println!();
            println!("#{} {}:", comment.id, comment.created_at);
            print!("{}", render::render_markdown(&comment.body).trim_end());
            println!();
        }
    }
}

fn join_ids(ids: &[i64]) -> String {
    ids.iter()
        .map(|id| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(", ")
}
