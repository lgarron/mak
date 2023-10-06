use async_std::task::{self, block_on, JoinHandle};
use futures::{future::join_all, FutureExt};
use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressStyle};
mod options;
use std::{
    collections::HashMap,
    fs::read_to_string,
    path::{Path, PathBuf},
    process::{exit, Command},
    sync::Arc,
    time::{Duration, Instant},
};

use options::get_options;
use parse::TargetName;

use crate::parse::TargetGraph;

mod parse;

fn main() {
    let start_time = Instant::now();
    let options = get_options();

    let makefile_path = options.makefile_path.unwrap_or("Makefile".into());
    let makefile_contents = read_to_string(&makefile_path).unwrap_or_else(|_| {
        if options.print_targets {
            exit(0);
        }

        println!("No Makefile specified and no file found called `Makefile`");
        println!("For more details, run: mak -h");
        exit(0);
    });
    let target_graph: TargetGraph =
        TargetGraph::try_from(&makefile_contents).expect("Could not parse Makefile");

    if options.print_graph {
        println!(
            "{}",
            serde_json::to_string_pretty(&target_graph).expect("Could not print graph")
        );
        exit(0)
    }
    if options.print_targets {
        let lines: Vec<String> = target_graph
            .0
            .keys()
            .map(|target_name| target_name.to_string())
            .collect();
        println!("{}", lines.join("\n"));
        exit(0)
    }

    let target_names: Vec<TargetName> = if options.targets.is_empty() {
        let default_target_name = match target_graph.0.keys().next() {
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
                if !target_graph.0.contains_key(&target_name) {
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
        makefile_path,
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
    makefile_path: PathBuf,
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
            .0
            .get(target_name)
            .expect("Internal error: Unexpectedly missing a target")
            .clone();
        let dependency_handles: Vec<SharedFuture> = dependencies
            .iter()
            .map(|target_name| (self.make_target(target_name, depth + 1)))
            .collect();
        let makefile_path_owned = self.makefile_path.to_owned();
        let target_name_owned = target_name.clone();
        let multi_progress_owned = self.multi_progress.clone();

        let progress_bar = ProgressBar::new(2);
        let progress_bar = multi_progress_owned.insert_from_back(0, progress_bar);
        progress_bar.set_style(
            ProgressStyle::with_template("  |   ⋯ | {prefix:20}")
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
                ProgressStyle::with_template("{spinner} | {elapsed:>03} | {prefix:20}")
                    .expect("Could not construct progress bar."),
            );
            progress_bar.enable_steady_tick(Duration::from_millis(16));

            make_individual_dependency(dependencies, &makefile_path_owned, &target_name_owned);

            progress_bar.set_position(2);
            progress_bar.set_style(
                ProgressStyle::with_template("🎯| {elapsed:>03} | {prefix:20}")
                    .expect("Could not construct progress bar."),
            );
        });
        let join_handle = join_handle.shared();
        self.futures
            .insert(target_name.clone(), join_handle.clone());
        join_handle
    }
}

fn make_individual_dependency(
    dependencies: Vec<TargetName>,
    makefile_path: &Path,
    target_name: &TargetName,
) {
    let makefile_path_str = &makefile_path.to_string_lossy();
    let mut args = vec!["-f", makefile_path_str, &target_name.0];

    for dependency in &dependencies {
        args.push("-o");
        args.push(&dependency.0);
    }

    // println!("[{}] Starting…", target_name);
    let _ = Command::new("make")
        .args(args)
        .output()
        .expect("failed to execute process");
    // println!("[{}] Finished.", target_name);
    // dbg!(output);
}
