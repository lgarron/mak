use std::fmt::Display;

use indexmap::IndexMap;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_while, take_while1},
    combinator::{all_consuming, not, opt},
    multi::{many0, separated_list0},
    IResult,
};

use serde::Serialize;
#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize)]
pub(crate) struct TargetName(pub(crate) String);

impl Display for TargetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct TargetGraph {
    pub(crate) edges: IndexMap<TargetName, Vec<TargetName>>,
    pub(crate) default_goal: Option<TargetName>,
}

fn is_allowed_target_name_first_char(c: char) -> bool {
    !is_makefile_whitespace(c) && c != '\n' && c != '\r' && c != ':'
}

fn is_allowed_target_name_tail_char(c: char) -> bool {
    !is_makefile_whitespace(c) && c != '\n' && c != '\r' && c != ':'
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
    let (input, target_name_first_char) = take_while1(is_allowed_target_name_first_char)(input)?;
    let (input, target_name_tail) = take_while(is_allowed_target_name_tail_char)(input)?;
    let target_name = TargetName([target_name_first_char, target_name_tail].join(""));
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

    target_graph.edges.insert(target_name, dependencies);
    Ok((input, Some(target_graph)))
}

fn parse_default_goal(input: &str) -> IResult<&str, Option<TargetGraph>> {
    let (input, _) = tag(".DEFAULT_GOAL := ")(input)?;
    let (input, target_name) = parse_target_name(input)?;
    let target_graph = TargetGraph {
        edges: IndexMap::new(),
        default_goal: Some(target_name),
    };
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
        alt((
            parse_makefile_target,
            parse_default_goal,
            parse_ignored_line,
        )),
    )(input)?;
    for target_graph in target_graphs.into_iter().flatten() {
        main_target_graph.edges.extend(target_graph.edges);
        if let Some(default_goal) = target_graph.default_goal {
            main_target_graph.default_goal = Some(default_goal); // TODO: test against multiple default goals?
        }
    }

    Ok((input, main_target_graph))
}

impl TryFrom<&String> for TargetGraph {
    type Error = String;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        match all_consuming(parse_makefile)(value) {
            Ok((_, target_graph)) => Ok(target_graph),
            Err(e) => {
                eprintln!("Makefile parsing error: {:#?}", e);
                Err("Invalid Makefile".into()) // TODO: pass on error
            }
        }
    }
}
