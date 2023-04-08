use std::{path::Path, str::FromStr};

use nom::{
    branch::alt,
    bytes::complete::{escaped, is_not, tag},
    character::complete::{alpha1, alphanumeric1, multispace0},
    combinator::{map, recognize},
    error::context,
    multi::{many0, many1},
    sequence::{delimited, pair, separated_pair},
    IResult,
};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::loader::stellaris_documents_dir;

// Define the ModDefinition struct to hold the parsed values
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ModDefinition {
    pub version: Option<String>,
    pub tags: Vec<String>,
    pub name: String,
    pub picture: Option<String>,
    pub supported_version: Option<String>,
    pub path: Option<String>,
    pub remote_file_id: Option<String>,
    pub dependencies: Vec<String>,
    pub archive: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ModDefinitionList {
    /// The mods that were parsed and in the definition list
    pub mods: Vec<ModDefinition>,

    /// A list of files that failed to parse
    pub failed_parse_files: Vec<String>,
}

impl ModDefinitionList {
    pub fn new() -> Self {
        ModDefinitionList {
            mods: Vec::new(),
            failed_parse_files: Vec::new(),
        }
    }

    pub fn load_from_my_documents(custom_path: Option<&Path>) -> Result<Self, anyhow::Error> {
        let mut mod_dir = stellaris_documents_dir(custom_path)?;
        mod_dir.push("mod");

        let dot_mod_files = WalkDir::new(mod_dir)
            .max_depth(0)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| {
                if let Ok(e) = e {
                    if e.file_type().is_file() && e.path().extension().unwrap_or_default() == "mod"
                    {
                        return Some(e);
                    }
                }
                None
            });

        let mut definitions = ModDefinitionList::new();

        for item in dot_mod_files {
            let mod_definition = ModDefinition::load_from_file(&item.path());
            definitions.push(mod_definition);
        }

        Ok(definitions)
    }

    pub fn push(&mut self, definition: Result<ModDefinition, String>) -> &Self {
        match definition {
            Ok(definition) => self.mods.push(definition),
            Err(e) => self.failed_parse_files.push(e),
        }
        self
    }
}

impl ModDefinition {
    pub fn new() -> Self {
        ModDefinition {
            version: None,
            tags: Vec::new(),
            name: String::new(),
            picture: None,
            supported_version: None,
            path: None,
            remote_file_id: None,
            dependencies: Vec::new(),
            archive: None,
        }
    }

    pub fn load(input: &str) -> Result<Self, String> {
        let (_, mod_definition) = parse_mod_definition(input).map_err(|e| e.to_string())?;
        Ok(mod_definition)
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let contents = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::load(&contents)
    }
}

fn string_value(input: &str) -> IResult<&str, &str> {
    delimited(
        tag("\""),
        escaped(is_not("\\\""), '\\', tag("\"")),
        tag("\""),
    )(input)
}

fn array_value(input: &str) -> IResult<&str, Vec<&str>> {
    delimited(
        tag("{"),
        many1(delimited(multispace0, string_value, multispace0)),
        tag("}"),
    )(input)
}

// Define the parse_value function to parse string and array values
fn parse_value<'a>(input: &'a str) -> IResult<&'a str, Vec<String>> {
    alt((
        map(string_value, |s| vec![s.to_string()]),
        map(array_value, |v| v.iter().map(|s| s.to_string()).collect()),
    ))(input)
}

// Define the parse_expression function to parse a single key-value expression
fn parse_expression<'a>(input: &'a str) -> IResult<&'a str, (&str, Vec<String>)> {
    separated_pair(
        context("identifier", identifier),
        tag("="),
        context("value", parse_value),
    )(input)
}

// Define the parse_mod_definition function to parse the entire input string
fn parse_mod_definition<'a>(input: &'a str) -> IResult<&'a str, ModDefinition> {
    let (input, expressions) = many0(delimited(
        multispace0,
        context("expression", parse_expression),
        multispace0,
    ))(input)?;

    // Convert the list of expressions to a ModDefinition struct
    let mut mod_info = ModDefinition {
        version: None,
        tags: Vec::new(),
        name: String::new(),
        picture: None,
        supported_version: None,
        path: None,
        remote_file_id: None,
        archive: None,
        dependencies: Vec::new(),
    };
    for (key, value) in expressions {
        match key {
            "version" => mod_info.version = Some(value[0].clone()),
            "tags" => mod_info.tags = value.clone(),
            "name" => mod_info.name = value[0].clone(),
            "picture" => mod_info.picture = Some(value[0].clone()),
            "supported_version" => mod_info.supported_version = Some(value[0].clone()),
            "path" => mod_info.path = Some(value[0].clone()),
            "remote_file_id" => mod_info.remote_file_id = Some(value[0].clone()),
            "archive" => mod_info.archive = Some(value[0].clone()),
            "dependencies" => mod_info.dependencies = value.clone(),
            _ => (),
        }
    }
    Ok((input, mod_info))
}

// Define the identifier function to match identifier strings
fn identifier<'a>(input: &'a str) -> IResult<&'a str, &str> {
    context(
        "identifier",
        recognize(pair(
            alt((alpha1, tag("_"))),
            many0(alt((alphanumeric1, tag("_")))),
        )),
    )(input)
}

impl FromStr for ModDefinition {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_mod_definition(s)
            .map(|(_, mod_info)| mod_info)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mod_definition() {
        let input = r#"version="2.8"
        tags={
            "Technologies"
            "Economy"
            "Buildings"
        }
        name="EUTAB - Ethos Unique Techs and Buildings"
        picture="eutab.png"
        supported_version="3.0.*"
        path="D:/SteamLibrary/steamapps/workshop/content/281990/804732593"
        remote_file_id="804732593""#;
        let expected_output = ModDefinition {
            version: Some(String::from("2.8")),
            tags: vec![
                String::from("Technologies"),
                String::from("Economy"),
                String::from("Buildings"),
            ],
            name: String::from("EUTAB - Ethos Unique Techs and Buildings"),
            picture: Some(String::from("eutab.png")),
            supported_version: Some(String::from("3.0.*")),
            path: Some(String::from(
                "D:/SteamLibrary/steamapps/workshop/content/281990/804732593",
            )),
            remote_file_id: Some(String::from("804732593")),
            archive: None,
            dependencies: Vec::new(),
        };
        assert_eq!(parse_mod_definition(input), Ok(("", expected_output)));
    }

    #[test]
    fn test_parse_value() {
        let input = r#""Hello, world!""#;
        let expected_output = vec![String::from("Hello, world!")];
        assert_eq!(parse_value(input), Ok(("", expected_output)));

        let input = r#"{ "Technologies" "Economy" "Buildings" }"#;
        let expected_output = vec![
            String::from("Technologies"),
            String::from("Economy"),
            String::from("Buildings"),
        ];
        assert_eq!(parse_value(input), Ok(("", expected_output)));
    }

    #[test]
    fn test_parse_expression() {
        let input = r#"version="2.8""#;
        let expected_output = ("version", vec![String::from("2.8")]);
        assert_eq!(parse_expression(input), Ok(("", expected_output)));

        let input = r#"tags={ "Technologies" "Economy" "Buildings" }"#;
        let expected_output = (
            "tags",
            vec![
                String::from("Technologies"),
                String::from("Economy"),
                String::from("Buildings"),
            ],
        );
        assert_eq!(parse_expression(input), Ok(("", expected_output)));
    }

    #[test]
    fn test_identifier() {
        let input = "version";
        assert_eq!(identifier(input), Ok(("", input)));

        let input = "_my_identifier_123";
        assert_eq!(identifier(input), Ok(("", input)));

        let input = "123_identifier"; // This should fail because the identifier cannot start with a number
        assert!(identifier(input).is_err());
    }
}
