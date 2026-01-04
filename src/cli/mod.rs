#[derive(clap::Parser)]
#[command(name = "Gato")]
#[command(about = "A High-Performance, Parallelized Version Control System", long_about = None)]
pub struct Cli {
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
    Commit { message: String, author: String },
    #[clap(
        name = "status",
        about = "Show the working tree status (not implemented yet!)",
        alias = "s"
    )]
    Status,
    #[clap(
        name = "log",
        about = "Show commit logs (not implemented yet!)",
        alias = "l"
    )]
    Log,
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
}
