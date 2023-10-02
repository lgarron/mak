mod options;
use std::fs::{self};

use options::get_options;

use crate::parse::TargetGraph;

mod parse;

fn main() {
    let options = get_options();

    let makefile_path = options.makefile_path.unwrap_or("Makefile".into());
    let makefile_contents = fs::read_to_string(makefile_path).expect("Could not read Makefile");
    let target_graph: TargetGraph =
        TargetGraph::try_from(&makefile_contents).expect("Could not parse Makefile");

    dbg!(target_graph);
}
