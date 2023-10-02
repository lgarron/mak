mod options;
use std::{
    collections::HashMap,
    fs::{self},
    path::Path,
    process::{exit, Command},
    thread::spawn,
};

use options::get_options;
use parse::TargetName;
use spmc::Receiver;

use crate::parse::TargetGraph;

mod parse;

fn main() {
    let options = get_options();

    let makefile_path = options.makefile_path.unwrap_or("Makefile".into());
    let makefile_contents = fs::read_to_string(&makefile_path).unwrap_or_else(|_| {
        eprintln!("Could not read Makefile");
        exit(1)
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

    let default_target_name = match target_graph.0.keys().next() {
        Some(target_name) => target_name,
        None => {
            eprintln!("No target specified and no default target available");
            exit(1)
        }
    };
    let main_target_name = match options.target {
        Some(target_name) => {
            let target_name = TargetName(target_name);
            if !target_graph.0.contains_key(&target_name) {
                eprintln!("Unknown target specified: {}", target_name);
                exit(1)
            };
            target_name
        }
        None => default_target_name.clone(),
    };

    make_target(
        &mut HashMap::default(),
        &target_graph,
        &makefile_path,
        &main_target_name,
    )
    .recv()
    .expect("Main target did not build successfully.")
}

fn make_target(
    signals: &mut HashMap<TargetName, Receiver<()>>,
    target_graph: &TargetGraph,
    makefile_path: &Path,
    target_name: &TargetName,
) -> Receiver<()> {
    if let Some(receiver) = signals.get(target_name) {
        return receiver.clone();
    }
    let (mut tx, rx) = spmc::channel::<()>();
    signals.insert(target_name.clone(), rx.clone());

    let dependencies = target_graph
        .0
        .get(target_name)
        .expect("Internal error: Unexpectedly missing a target")
        .clone();
    let dependency_receivers: Vec<Receiver<()>> = dependencies
        .iter()
        .map(|target_name| make_target(signals, target_graph, makefile_path, target_name))
        .collect();
    let makefile_path_owned = makefile_path.to_owned();
    let target_name_owned = target_name.clone();

    spawn(move || {
        let target_name_owned = target_name_owned;
        for dependency_receiver in dependency_receivers {
            dependency_receiver
                .recv()
                .expect("A dependency did not build successfully.");
        }
        make_individual_dependency(dependencies, &makefile_path_owned, &target_name_owned);
        tx.send(())
            .expect("Internal error: could not coordinate dependencies");
    });
    rx
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

    let output = Command::new("make")
        .args(args)
        .output()
        .expect("failed to execute process");
    dbg!(output);
}
