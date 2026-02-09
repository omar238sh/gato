use std::path::PathBuf;

#[derive(clap::Parser)]
#[command(name = "Gato")]
#[command(about = "A High-Performance, Parallelized Version Control System", long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value = ".")]
    pub path: PathBuf,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    #[clap(
        name = "init",
        about = "Initialize a new Gato repository in the current directory",
        alias = "i"
    )]
    Init,
    #[clap(name = "add", about = "Add file contents to the index", alias = "a")]
    Add { paths: Vec<String> },
    #[clap(
        name = "commit",
        about = "Record changes to the repository",
        alias = "c"
    )]
    Commit { message: String },

    #[clap(
        name = "checkout",
        about = "Checkout a specific commit 0 for last commit",
        alias = "co"
    )]
    Checkout { commit_index: usize },
    #[clap(name = "new-branch", about = "Create a new branch", alias = "nb")]
    NewBranch { branch_name: String },
    #[clap(
        name = "change-branch",
        about = "Change to an existing branch",
        alias = "cb"
    )]
    ChangeBranch { branch_name: String },
    #[clap(
        name = "soft-reset",
        about = "Reset to a specific commit index",
        alias = "ci"
    )]
    SoftReset { commit_index: usize },

    #[clap(
        name = "gc",
        about = "Garbage collect unreferenced objects",
        alias = "gc"
    )]
    Gc,

    #[clap(
        name = "list-repos",
        about = "List all linked repositories",
        alias = "lr"
    )]
    ListRepos,

    #[clap(name = "delete-repo", about = "Delete a repository", alias = "dr")]
    DeleteRepo,

    #[clap(
        name = "delete-branch",
        about = "Delete a branch from the repository",
        alias = "db"
    )]
    DeleteBranch { name: String },

    #[clap(name = "status", about = "Show the working tree status", alias = "st")]
    Status,

    #[clap(
        name = "merge",
        about = "Merge a branch into the current branch",
        alias = "m"
    )]
    Merge {
        target_branch: String,
        message: String,
    },
}
