use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "issues",
    version,
    about = "Manage agent issues in a local SQLite store at .scratch/issues.db"
)]
pub struct Cli {
    /// Path to the SQLite database
    #[arg(long, global = true, env = "ISSUES_DB", value_name = "PATH")]
    pub db: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new issue
    Create(CreateArgs),
    /// List issues, optionally filtered by status or label
    List {
        /// Only show issues in this status (open, in-progress, blocked, closed)
        #[arg(long)]
        status: Option<String>,
        /// Only show issues carrying this label
        #[arg(long)]
        label: Option<String>,
    },
    /// Show a single issue in detail
    Get { id: i64 },
    /// Update an issue (e.g. its status)
    Update(UpdateArgs),
    /// Advance the frontier: open issues whose dependencies are all closed
    Frontier {
        /// Only show issues carrying this label (e.g. wayfinder:research)
        #[arg(long)]
        label: Option<String>,
    },
    /// Show open issues blocked by an unresolved dependency
    Blocked {
        /// Only show issues carrying this label
        #[arg(long)]
        label: Option<String>,
    },
    /// List open issues labelled ready-for-agent (the agent grab queue)
    Ready,
    /// Write the docs/agents/issue-tracker.md agent instructions
    AgentInstructions {
        /// Output path (default: docs/agents/issue-tracker.md)
        #[arg(long, value_name = "PATH")]
        output: Option<PathBuf>,
    },
    /// Add or remove a label on an issue
    Label {
        id: i64,
        #[command(subcommand)]
        action: LabelAction,
    },
    /// Close an issue
    Close { id: i64 },
    /// Add a comment to an issue
    Comment(CommentArgs),
    /// Add a dependency edge (issue depends on another issue)
    Depends(DependsArgs),
}

#[derive(Args)]
pub struct CreateArgs {
    /// Issue title
    #[arg(long)]
    pub title: String,
    /// Path to a file whose contents become the issue body
    #[arg(long, value_name = "PATH")]
    pub body_file: Option<PathBuf>,
    /// Inline issue body (mutually exclusive with --body-file)
    #[arg(long)]
    pub body: Option<String>,
}

#[derive(Args)]
pub struct UpdateArgs {
    pub id: i64,
    /// New status (open, in-progress, blocked, closed)
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(Subcommand)]
pub enum LabelAction {
    /// Attach a label to the issue
    Add { name: String },
    /// Detach a label from the issue
    Remove { name: String },
}

#[derive(Args)]
pub struct CommentArgs {
    pub id: i64,
    /// Comment body
    #[arg(long)]
    pub body: String,
}

#[derive(Args)]
pub struct DependsArgs {
    pub id: i64,
    /// Id of an issue this issue depends on (repeatable)
    #[arg(long = "on", value_name = "DEPENDENCY_ID")]
    pub on: Vec<i64>,
}
