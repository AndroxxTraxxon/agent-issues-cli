use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use issues::cli::{Cli, Command, LabelAction};
use issues::db::{self, BlockedSummary, IssueDetail, IssueSummary};
use issues::instructions;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db_path = cli
        .db
        .unwrap_or_else(|| PathBuf::from(".scratch/issues.db"));
    let conn =
        db::open(&db_path).with_context(|| format!("opening database at {}", db_path.display()))?;

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
            let issue = db::create_issue(&conn, &args.title, &body)?;
            println!("Created issue #{}: {}", issue.id, issue.title);
        }
        Command::List { status, label } => {
            let issues = db::list_issues(&conn, status.as_deref(), label.as_deref())?;
            print_list(&issues);
        }
        Command::Get { id } => match db::get_issue(&conn, id)? {
            Some(detail) => print_detail(&detail),
            None => anyhow::bail!("issue {id} not found"),
        },
        Command::Frontier { label } => {
            let issues = db::frontier(&conn, label.as_deref())?;
            print_list(&issues);
        }
        Command::Blocked { label } => {
            let issues = db::blocked(&conn, label.as_deref())?;
            print_blocked(&issues);
        }
        Command::Ready => {
            let issues = db::ready_for_agent(&conn)?;
            print_list(&issues);
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
            let issue = db::update_status(
                &conn,
                args.id,
                &args.status.context("--status is required")?,
            )?;
            println!(
                "Issue #{}: {} is now {}",
                issue.id, issue.title, issue.status
            );
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
        Command::Close { id } => {
            let issue = db::close_issue(&conn, id)?;
            println!("Closed issue #{}: {}", issue.id, issue.title);
        }
        Command::Comment(args) => {
            let comment = db::add_comment(&conn, args.id, &args.body)?;
            println!("Added comment #{} to issue #{}", comment.id, args.id);
        }
        Command::Depends(args) => {
            if args.on.is_empty() {
                println!("No dependencies given (use --on <DEPENDENCY_ID>)");
            }
            for dependency_id in &args.on {
                db::add_dependency(&conn, args.id, *dependency_id)?;
                println!("Issue #{} now depends on #{dependency_id}", args.id);
            }
        }
    }

    Ok(())
}

fn print_list(issues: &[IssueSummary]) {
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
        println!(
            "{:<id_w$}  {:<status_w$}  {:<labels_w$}  {}",
            issue.id,
            issue.status,
            label_text(&issue.labels),
            issue.title,
        );
    }
    println!("{} issue(s).", issues.len());
}

fn print_blocked(issues: &[BlockedSummary]) {
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
        .map(|i| blocked_text(i).len())
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
            blocked_text(issue),
        );
    }
    println!("{} blocked issue(s).", issues.len());
}

fn blocked_text(issue: &BlockedSummary) -> String {
    issue
        .blocked_by
        .iter()
        .map(|d| format!("#{} {} ({})", d.id, d.title, d.status))
        .collect::<Vec<_>>()
        .join(", ")
}

fn label_text(labels: &[String]) -> String {
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels.join(", ")
    }
}

fn print_detail(detail: &IssueDetail) {
    let issue = &detail.issue;
    println!("Issue #{}: {}", issue.id, issue.title);
    println!("Status: {}", issue.status);
    println!("Labels: {}", label_text(&detail.labels));
    println!("Created: {}", issue.created_at);
    println!("Updated: {}", issue.updated_at);

    let blocked_by = join_ids(&detail.blocked_by);
    let blocks = join_ids(&detail.blocks);
    if !blocked_by.is_empty() {
        println!("\nBlocked by: {}", blocked_by);
    }
    if !blocks.is_empty() {
        println!("Blocks: {}", blocks);
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

fn join_ids(ids: &[i64]) -> String {
    ids.iter()
        .map(|id| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(", ")
}
