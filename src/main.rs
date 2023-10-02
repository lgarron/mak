mod options;
use std::{
    fs::{self},
    process::exit,
};

use options::get_options;

use crate::parse::TargetGraph;

mod parse;

fn main() {
    let options = get_options();

    let makefile_path = options.makefile_path.unwrap_or("Makefile".into());
    let makefile_contents = fs::read_to_string(makefile_path).expect("Could not read Makefile");
    let target_graph: TargetGraph =
        TargetGraph::try_from(&makefile_contents).expect("Could not parse Makefile");

    if options.print_graph {
        println!(
            "{}",
            serde_json::to_string_pretty(&target_graph).expect("Could not print graph")
        );
        exit(0)
    }
}
