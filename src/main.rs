use async_std::task::{self, block_on, JoinHandle};
use futures::{future::join_all, FutureExt};
use indexmap::IndexMap;
use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressStyle};
mod options;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    path::Path,
    process::{exit, Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

use options::{get_options, MakArgs};
use parse::TargetName;

use crate::parse::TargetGraph;

mod parse;

const ERROR_COULD_NOT_LIST_TARGETS: &str =
    "Could not list targets using `make` (are you missing a Makefile?)";

fn makefile_not_found(options: &MakArgs) {
    if options.print_completion_targets {
        exit(0);
    }
    eprintln!("No Makefile specified and no file found called `Makefile`");
    eprintln!("For more details, run: mak -h");
    exit(0);
}

fn main() {
    let start_time = Instant::now();
    let options = get_options();

    let mut args = vec!["-pRrq".to_owned()];
    let makefile_path_str = options.makefile_path.as_ref().map(|p| {
        p.to_str()
            .expect("Could not convert Makefile path to a string.")
            .to_owned()
    });
    if let Some(some_makefile_path_str) = &makefile_path_str {
        let path = Path::new(&some_makefile_path_str);
        if !path.exists() {
            makefile_not_found(&options);
        }
        args.append(&mut make_args(&makefile_path_str));
    } else if !Path::new("makefile").exists() && !Path::new("Makefile").exists() {
        makefile_not_found(&options);
    }

    let child = Command::new("make")
        .args(args)
        .stdout(Stdio::piped())
        .spawn()
        .expect(ERROR_COULD_NOT_LIST_TARGETS);
    let output = child
        .wait_with_output()
        .expect(ERROR_COULD_NOT_LIST_TARGETS);

    let stdout_str = String::from_utf8(output.stdout).expect(ERROR_COULD_NOT_LIST_TARGETS);
    let mut target_graph: TargetGraph =
        TargetGraph::try_from(&stdout_str).expect("Could not parse targets");
    target_graph.edges = IndexMap::from_iter(target_graph.edges.into_iter().filter(|edge| {
        !edge.0 .0.starts_with('.') && makefile_path_str != Some(edge.0 .0.clone())
    }));

    if options.print_graph {
        println!(
            "{}",
            serde_json::to_string_pretty(&target_graph).expect("Could not print graph")
        );
        exit(0)
    }
    if options.print_completion_targets {
        let lines: Vec<String> = target_graph
            .edges
            .keys()
            .map(|target_name| target_name.to_string())
            .collect();
        println!("{}", lines.join("\n"));
        exit(0)
    }

    let target_names: Vec<TargetName> = if options.targets.is_empty() {
        let default_target_name = match &target_graph.default_goal {
            Some(target_name) => target_name.clone(),
            None => {
                eprintln!("No target specified and no default target available");
                exit(1)
            }
        };
        vec![default_target_name]
    } else {
        options
            .targets
            .iter()
            .map(|target_string| {
                let target_name = TargetName(target_string.to_owned());
                if !target_graph.edges.contains_key(&target_name) {
                    eprintln!("Unknown target specified: {}", target_name);
                    exit(1)
                };
                target_name
            })
            .collect()
    };

    let multi_progress = Arc::new(MultiProgress::new());

    let mut shared_make = SharedMake {
        multi_progress: multi_progress.clone(),
        futures: HashMap::default(),
        target_graph,
        makefile_path_str,
    };

    block_on(shared_make.make_targets(&target_names));
    let num_main_targets = target_names.len();
    let num_dependencies = shared_make.futures.len() - num_main_targets;
    if options.dry_run {
        println!(
            "Dry run found {} target{} and {} additional dependenc{} in {:?}",
            num_main_targets,
            if num_main_targets == 1 { "" } else { "s" },
            num_dependencies,
            if num_dependencies == 1 { "y" } else { "ies" },
            Instant::now() - start_time
        );
    } else {
        println!(
            "Built {} target{} and {} additional dependenc{} in {:?}",
            num_main_targets,
            if num_main_targets == 1 { "" } else { "s" },
            num_dependencies,
            if num_dependencies == 1 { "y" } else { "ies" },
            Instant::now() - start_time
        );
    }
}

type SharedFuture = futures::future::Shared<JoinHandle<()>>;

struct SharedMake {
    multi_progress: Arc<MultiProgress>,
    futures: HashMap<TargetName, SharedFuture>,
    target_graph: TargetGraph,
    makefile_path_str: Option<String>,
}

impl SharedMake {
    async fn make_targets(&mut self, target_names: &[TargetName]) {
        join_all(
            target_names
                .iter()
                .map(|target_name| self.make_target(target_name, 0)),
        )
        .await;
    }

    fn make_target(&mut self, target_name: &TargetName, depth: usize) -> SharedFuture {
        if let Some(sender) = self.futures.get(target_name) {
            // TODO: update depth if it decreased?
            return sender.clone();
        }

        let dependencies = self
            .target_graph
            .edges
            .get(target_name)
            .expect("Internal error: Unexpectedly missing a target")
            .clone();
        let dependency_handles: Vec<SharedFuture> = dependencies
            .iter()
            .map(|target_name| (self.make_target(target_name, depth + 1)))
            .collect();
        let makefile_path_str_owned = self.makefile_path_str.to_owned();
        let target_name_owned = target_name.clone();
        let multi_progress_owned = self.multi_progress.clone();

        let progress_bar = ProgressBar::new(2);
        let progress_bar = multi_progress_owned.insert_from_back(0, progress_bar);
        progress_bar.set_style(
            ProgressStyle::with_template("  |   ⋯ | {prefix}")
                .expect("Could not construct progress bar."),
        );
        let progress_bar = progress_bar.with_finish(ProgressFinish::AndLeave);
        let indentation = match depth {
            0 => "".to_owned(),
            depth => format!("{}{} ", " ".repeat(depth - 1), "↙"),
        };
        progress_bar.set_prefix(format!("{}{}", indentation, target_name_owned));
        progress_bar.set_position(0);
        let join_handle = task::spawn(async move {
            join_all(dependency_handles).await;

            progress_bar.reset_elapsed();
            progress_bar.set_position(1);
            progress_bar.set_style(
                ProgressStyle::with_template(
                    "{spinner} | {elapsed:>03} | {prefix:40} 🛠️ | {wide_msg}",
                )
                .expect("Could not construct progress bar."),
            );
            progress_bar.enable_steady_tick(Duration::from_millis(16));

            make_individual_dependency(
                dependencies,
                &makefile_path_str_owned,
                &target_name_owned,
                &progress_bar,
            )
            .await;

            progress_bar.set_position(2);
            progress_bar.set_style(
                ProgressStyle::with_template("🎯| {elapsed:>03} | {prefix}")
                    .expect("Could not construct progress bar."),
            );
        });
        let join_handle = join_handle.shared();
        self.futures
            .insert(target_name.clone(), join_handle.clone());
        join_handle
    }
}

async fn make_individual_dependency(
    dependencies: Vec<TargetName>,
    makefile_path_str: &Option<String>,
    target_name: &TargetName,
    progress_bar: &ProgressBar,
) {
    let mut args = make_args(makefile_path_str);
    args.push(target_name.0.clone());

    for dependency in &dependencies {
        args.push("-o".to_owned());
        args.push(dependency.0.clone());
    }
    args.push("--".to_owned());

    let child = Command::new("make")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    // TODO: deduplicate stderr and stdout implementations.
    let stdout_reader = BufReader::new(
        child
            .stdout
            .expect("Could not get stdout for a `make` invocation."),
    );
    let stdout_progress_bar_clone: ProgressBar = progress_bar.clone();
    let stdout_handle: JoinHandle<()> = task::spawn(async move {
        stdout_reader
            .lines()
            .map_while(Result::ok)
            .for_each(|line| {
                if !line.trim().is_empty() {
                    stdout_progress_bar_clone.set_message(line)
                }
            });
    });

    let stderr_reader = BufReader::new(
        child
            .stderr
            .expect("Could not get stdout for a `make` invocation."),
    );
    let stderr_progress_bar_clone: ProgressBar = progress_bar.clone();
    let stderr_handle: JoinHandle<()> = task::spawn(async move {
        stderr_reader
            .lines()
            .map_while(Result::ok)
            .for_each(|line| {
                if !line.trim().is_empty() {
                    stderr_progress_bar_clone.set_message(line)
                }
            });
    });
    join_all([stdout_handle, stderr_handle]).await;
}

fn make_args(makefile_path_str: &Option<String>) -> Vec<String> {
    let mut args = vec![];
    if let Some(makefile_path_str) = makefile_path_str {
        args.push("-f".to_owned());
        args.push(makefile_path_str.to_owned());
    };
    args
}
