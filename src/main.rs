mod options;
use std::{
    fs::{self},
    path::Path,
    process::{exit, Command},
};

use options::get_options;
use parse::TargetName;

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

    target_graph.make_individual_dependency(main_target_name, &makefile_path);
}

impl TargetGraph {
    fn make_individual_dependency(&self, target_name: TargetName, makefile_path: &Path) {
        let makefile_path_str = &makefile_path.to_string_lossy();
        let mut args = vec!["-f", makefile_path_str, &target_name.0];

        for dependency in self
            .0
            .get(&target_name)
            .expect("Unexpectedly missing target")
        {
            args.push("-o");
            args.push(&dependency.0);
        }

        let output = Command::new("make")
            .args(args)
            .output()
            .expect("failed to execute process");
        dbg!(output);
    }
}
