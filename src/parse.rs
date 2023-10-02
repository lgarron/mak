use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_while1},
    character::complete::line_ending,
    combinator::all_consuming,
    multi::many0,
    IResult,
};

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct TargetName(String);

#[derive(Debug, Default)]
pub(crate) struct TargetGraph(HashMap<TargetName, Vec<TargetName>>);

fn is_allowed_target_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn parse_makefile_target(input: &str) -> IResult<&str, Option<TargetGraph>> {
    let (input, target_name) = take_while1(is_allowed_target_name_char)(input)?;
    let target_name = TargetName(target_name.to_owned());

    let mut target_graph = TargetGraph::default();
    target_graph.0.insert(target_name, vec![]);

    let (input, _) = tag(":")(input)?;
    let (input, _) = take_till(|c| c == '\n')(input)?;
    let (input, _) = line_ending(input)?;

    Ok((input, Some(target_graph)))
}

// fn char_is_newline(c: char) -> bool {
//     is_newline(c)
// }

fn parse_ignored_line(input: &str) -> IResult<&str, Option<TargetGraph>> {
    let (input, _) = take_till(|c| c == '\n')(input)?;
    let (input, _) = line_ending(input)?;
    Ok((input, None))
}

fn parse_alg(input: &str) -> IResult<&str, TargetGraph> {
    let mut main_target_graph = TargetGraph::default();

    let (input, target_graphs) = many0(alt((parse_makefile_target, parse_ignored_line)))(input)?;
    for target_graph in target_graphs.into_iter().flatten() {
        main_target_graph.0.extend(target_graph.0);
    }

    Ok((input, main_target_graph))
}

impl TryFrom<&String> for TargetGraph {
    type Error = String;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        match all_consuming(parse_alg)(value) {
            Ok((_, target_graph)) => Ok(target_graph),
            Err(s) => {
                eprintln!("Makefile parsing error: {}", s);
                Err("Invalid Makefile".into()) // TODO: pass on error
            }
        }
    }
}
