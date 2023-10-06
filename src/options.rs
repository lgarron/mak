use clap::{CommandFactory, Parser};
use clap_complete::generator::generate;
use clap_complete::{Generator, Shell};
use std::io::stdout;
use std::path::PathBuf;
use std::process::exit;

/// Fast make
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[clap(name = "fake")]
pub(crate) struct FakeArgs {
    /// Makefile path
    #[clap(short = 'f', verbatim_doc_comment)]
    pub(crate) makefile_path: Option<PathBuf>,

    /// Makefile target
    #[clap(verbatim_doc_comment)]
    pub(crate) targets: Vec<String>, // TODO: `Vec<TargetName>`

    /// Show how commands would have been run, without actually running.
    #[clap(long, verbatim_doc_comment)]
    pub(crate) dry_run: bool,

    /// Print the dependency graph as JSON instead of running anything.
    #[clap(long, verbatim_doc_comment)]
    pub(crate) print_graph: bool,

    /// Print completions for the given shell (instead of generating any icons).
    /// These can be loaded/stored permanently (e.g. when using Homebrew), but they can also be sourced directly, e.g.:
    ///
    ///  folderify --completions fish | source # fish
    ///  source <(folderify --completions zsh) # zsh
    #[clap(long, verbatim_doc_comment, id = "SHELL")]
    pub(crate) completions: Option<Shell>,
}

fn completions_for_shell(cmd: &mut clap::Command, generator: impl Generator) {
    generate(generator, cmd, "fake", &mut stdout());
}

pub(crate) fn get_options() -> FakeArgs {
    let mut command = FakeArgs::command();

    let args = FakeArgs::parse();
    if let Some(shell) = args.completions {
        completions_for_shell(&mut command, shell);
        exit(0);
    }

    args
}
