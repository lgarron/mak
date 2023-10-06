use clap::{CommandFactory, Parser};
use clap_complete::generator::generate;
use clap_complete::{Generator, Shell};
use std::io::stdout;
use std::path::PathBuf;
use std::process::exit;

/// Fast make
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[clap(name = "mak")]
pub(crate) struct MakArgs {
    /// Makefile path
    #[clap(short = 'f', long = "file", alias = "makefile", verbatim_doc_comment)]
    pub(crate) makefile_path: Option<PathBuf>,

    /// Makefile target
    #[clap(verbatim_doc_comment)]
    pub(crate) targets: Vec<String>, // TODO: `Vec<TargetName>`

    /// Show how commands would have been run, without actually running.
    #[clap(long, group = "command-like", verbatim_doc_comment)]
    pub(crate) dry_run: bool,

    /// Print the dependency graph as JSON (instead of running anything).
    #[clap(long, group = "command-like", verbatim_doc_comment)]
    pub(crate) print_graph: bool,

    /// Print the the list of targets, one per line (instead of running anything).
    #[clap(long, group = "command-like", verbatim_doc_comment)]
    pub(crate) print_targets: bool,

    /// Print completions for the given shell (instead of running anything).
    /// These can be loaded/stored permanently (e.g. when using Homebrew), but they can also be sourced directly, e.g.:
    ///
    ///  mak --completions fish | source # fish
    ///  source <(mak --completions zsh) # zsh
    #[clap(long, group = "command-like", verbatim_doc_comment, id = "SHELL")]
    pub(crate) completions: Option<Shell>,
}

fn completions_for_shell(cmd: &mut clap::Command, generator: impl Generator) {
    generate(generator, cmd, "mak", &mut stdout());
}

pub(crate) fn get_options() -> MakArgs {
    let mut command = MakArgs::command();

    let args = MakArgs::parse();
    if let Some(shell) = args.completions {
        completions_for_shell(&mut command, shell);
        // TODO: other shells?
        if shell == Shell::Fish {
            // Complete targets for `fish` similarly to https://github.com/fish-shell/fish-shell/blob/3ce67ecbd2348fbe13e86a00bea6ce998710729a/share/completions/make.fish
            println!("
function __fish_complete_mak_targets
    # TODO: handle `-f=`?
    set -l file (string replace -rf '^mak .*((-f|--file)(=| +))([^ ]*) .*$' '$4' -- $argv)
    if test -n \"$file\"
        mak --file \"$file\" --print-targets
    else
        mak --print-targets
    end
end
complete -c mak -n 'commandline -ct | string match -q \"*=*\"' -a \"(__fish_complete_mak_targets (commandline -p))\" -d Target
complete -f -c mak -n 'commandline -ct | not string match -q \"*=*\"' -a \"(__fish_complete_mak_targets (commandline -p))\" -d Target
");
        }
        exit(0);
    }

    args
}
