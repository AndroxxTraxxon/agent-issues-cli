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

    /// Human view: render markdown and colorize output (only on a terminal)
    #[arg(long, global = true, visible_alias = "human")]
    pub pretty: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new issue
    Create(CreateArgs),
    /// List issues, optionally filtered by status, label, or parent
    List {
        /// Only show issues in this status (open, in-progress, blocked, closed)
        #[arg(long)]
        status: Option<String>,
        /// Only show issues carrying this label
        #[arg(long)]
        label: Option<String>,
        /// Only show issues attached to this parent
        #[arg(long)]
        parent: Option<i64>,
    },
    /// Show a single issue in detail
    Get { id: i64 },
    /// Update an issue (status, title, or body)
    Update(UpdateArgs),
    /// Advance the frontier: open issues whose dependencies are all closed
    Frontier {
        /// Only show issues carrying this label (e.g. wayfinder:research)
        #[arg(long)]
        label: Option<String>,
        /// Only show issues attached to this parent map
        #[arg(long)]
        map: Option<i64>,
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
    Close {
        id: i64,
        /// Optional resolution comment to record before closing
        #[arg(long)]
        comment: Option<String>,
    },
    /// Add a comment to an issue
    Comment(CommentArgs),
    /// Add or remove a blocking dependency edge
    Depends {
        #[command(subcommand)]
        action: DependsAction,
    },
    /// Attach an issue to a parent (membership, not a blocker)
    Attach(MembershipArgs),
    /// Detach an issue from a parent
    Detach(MembershipArgs),
}

#[derive(Subcommand)]
pub enum DependsAction {
    /// Add a dependency edge (issue depends on another issue)
    Add(DependsArgs),
    /// Remove a dependency edge
    Remove(DependsArgs),
}

#[derive(Args)]
pub struct DependsArgs {
    pub id: i64,
    /// Id of an issue this issue depends on (repeatable)
    #[arg(long = "on", value_name = "DEPENDENCY_ID")]
    pub on: Vec<i64>,
}

#[derive(Args)]
pub struct MembershipArgs {
    pub id: i64,
    /// Id of the parent issue (repeatable)
    #[arg(long = "parent", value_name = "PARENT_ID")]
    pub parent: Vec<i64>,
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
    /// New title
    #[arg(long)]
    pub title: Option<String>,
    /// Replace the issue body
    #[arg(long)]
    pub body: Option<String>,
    /// Path to a file whose contents replace the issue body
    #[arg(long, value_name = "PATH")]
    pub body_file: Option<PathBuf>,
    /// Append text to the issue body
    #[arg(long)]
    pub append_body: Option<String>,
    /// Path to a file whose contents are appended to the issue body
    #[arg(long, value_name = "PATH")]
    pub append_body_file: Option<PathBuf>,
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
    pub body: Option<String>,
    /// Path to a file whose contents become the comment body
    #[arg(long, value_name = "PATH")]
    pub body_file: Option<PathBuf>,
}
