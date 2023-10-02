use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_while, take_while1},
    combinator::{all_consuming, not, opt},
    multi::{many0, separated_list0},
    IResult,
};

use serde::Serialize;
#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub(crate) struct TargetName(String);

#[derive(Debug, Default, Serialize)]
pub(crate) struct TargetGraph(HashMap<TargetName, Vec<TargetName>>);

fn is_allowed_target_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn is_makefile_whitespace(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn parse_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = tag("#")(input)?;
    let (input, _) = take_till(|c| c == '\n')(input)?;
    Ok((input, ()))
}

fn parse_optional_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = opt(parse_comment)(input)?;
    Ok((input, ()))
}

fn parse_target_name(input: &str) -> IResult<&str, TargetName> {
    let (input, target_name) = take_while1(is_allowed_target_name_char)(input)?;
    let target_name = TargetName(target_name.to_owned());
    Ok((input, target_name))
}

// Starts with optional whitespace
fn parse_dependency(input: &str) -> IResult<&str, TargetName> {
    let (input, _) = many0(alt((
        tag(" "),
        tag("\t"),
        tag("|"),
        tag("\\\n"),
        tag("\\\r\n"),
    )))(input)?;
    parse_target_name(input)
}

fn target_name_with_colon(input: &str) -> IResult<&str, TargetName> {
    let (input, target_name) = parse_target_name(input)?;
    let (input, _) = tag(":")(input)?;
    Ok((input, target_name))
}

fn parse_makefile_target(input: &str) -> IResult<&str, Option<TargetGraph>> {
    let (input, target_name) = target_name_with_colon(input)?;
    let mut target_graph = TargetGraph::default();

    let (input, dependencies) = many0(parse_dependency)(input)?;

    let (input, _) = take_while(is_makefile_whitespace)(input)?;
    let (input, _) = parse_optional_comment(input)?;

    target_graph.0.insert(target_name, dependencies);
    Ok((input, Some(target_graph)))
}

fn parse_ignored_line(input: &str) -> IResult<&str, Option<TargetGraph>> {
    not(target_name_with_colon)(input)?;
    let (input, _) = take_till(|c| c == '\n')(input)?;
    Ok((input, None))
}

fn parse_makefile(input: &str) -> IResult<&str, TargetGraph> {
    let mut main_target_graph = TargetGraph::default();

    // TODO: fail on something that looks like a target declaration without valid deps.
    let (input, target_graphs) = separated_list0(
        alt((tag("\n"), tag("\r\n"))),
        alt((parse_makefile_target, parse_ignored_line)),
    )(input)?;
    for target_graph in target_graphs.into_iter().flatten() {
        main_target_graph.0.extend(target_graph.0);
    }

    Ok((input, main_target_graph))
}

impl TryFrom<&String> for TargetGraph {
    type Error = String;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        match all_consuming(parse_makefile)(value) {
            Ok((_, target_graph)) => Ok(target_graph),
            Err(s) => {
                eprintln!("Makefile parsing error: {}", s);
                Err("Invalid Makefile".into()) // TODO: pass on error
            }
        }
    }
}
