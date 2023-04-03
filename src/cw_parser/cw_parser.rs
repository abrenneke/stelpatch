use crate::cw_model::Module;
use anyhow::anyhow;
use path_slash::PathExt;
use std::{collections::HashMap, path::PathBuf};

use nom::{
    branch::alt,
    bytes::complete::{escaped, is_not, tag, take_until1},
    character,
    character::complete::{alphanumeric1, char, digit1, multispace0, multispace1, one_of, space1},
    combinator::{map_res, opt, recognize, value},
    error::context,
    multi::{many0, many1},
    sequence::{delimited, pair, tuple},
    IResult,
};

use crate::cw_model;

type Error<'a> = nom::error::Error<&'a str>;

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F, O>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, Error<'a>>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, Error<'a>>,
{
    delimited(multispace0, inner, multispace0)
}

/// Comments using #
pub fn comment<'a>(i: &'a str) -> IResult<&'a str, (), Error<'a>> {
    context(
        "comment",
        value(
            (), // Output is thrown away.
            pair(char('#'), opt(is_not("\n\r"))),
        ),
    )(i)
}

fn valid_identifier_char<'a>(input: &'a str) -> IResult<&'a str, &str, Error<'a>> {
    alt((
        alphanumeric1,
        tag("_"),
        tag("."),
        tag(":"),
        tag("@"),
        tag("-"),
        tag("|"),
        tag("/"),
        tag("\\"),
        tag("$"),
        tag("'"),
    ))(input)
}

fn identifier<'a>(input: &'a str) -> IResult<&'a str, &str, Error<'a>> {
    context("identifier", recognize(many1(valid_identifier_char)))(input)
}

#[derive(Debug)]
struct Expression {
    is_define: bool,
    key: String,
    operator: cw_model::Operator,
    value: cw_model::Value,
}

/// Identifier like identifier_name
fn expression<'a>(input: &'a str) -> IResult<&'a str, Expression, Error<'a>> {
    let (input, _) = skip_comments_and_blank_lines(input)?;
    let (input, key) = identifier_or_quoted_string(input)?;
    // dbg!(key);
    let (input, op) = ws(operator)(input)?;
    let (input, value) = ws(script_value)(input).expect("Failed to parse value");
    let (input, _) = skip_comments_and_blank_lines(input)?;

    Ok((
        input,
        Expression {
            key: key.to_string(),
            operator: op,
            value,
            is_define: false,
        },
    ))
}

/// An operator like >, <, >=, <=
fn operator<'a>(input: &'a str) -> IResult<&'a str, cw_model::Operator, Error<'a>> {
    let (input, op) = map_res(
        alt((
            tag(">="),
            tag("<="),
            tag("!="),
            tag("="),
            tag(">"),
            tag("<"),
        )),
        |v: &str| v.parse::<cw_model::Operator>(),
    )(input)?;
    Ok((input, op))
}

fn identifier_or_quoted_string<'a>(input: &'a str) -> IResult<&'a str, &str, Error<'a>> {
    ws(alt((identifier, quoted_value)))(input)
}

/// A value of a key-value pair, value-like: 1, 1.0, {}, identifier_like
fn script_value<'a>(input: &'a str) -> IResult<&'a str, cw_model::Value, Error<'a>> {
    let (input, v) = ws(alt((
        empty_entity,
        entity,
        array,
        map_res(decimal, |v| Ok::<_, Error<'a>>(cw_model::Value::Float(v))),
        map_res(integer, |v| Ok::<_, Error<'a>>(cw_model::Value::Integer(v))),
        rgb,
        hsv,
        map_res(define_identifier, |v| {
            Ok::<_, Error<'a>>(cw_model::Value::Define(v.to_string()))
        }),
        map_res(identifier_or_quoted_string, |v| {
            Ok::<_, Error<'a>>(cw_model::Value::String(v.to_string()))
        }),
    )))(input)?;
    Ok((input, v))
}

fn quoted_value<'a>(input: &'a str) -> IResult<&'a str, &str, Error<'a>> {
    context(
        "quoted_value",
        alt((
            delimited(
                char('"'),
                escaped(is_not("\\\""), '\\', one_of("\"")),
                char('"'),
            ),
            map_res(tag("\"\""), |_| Ok::<_, Error<'a>>("")),
        )),
    )(input)
}

fn array_value<'a>(input: &'a str) -> IResult<&'a str, cw_model::Value, Error<'a>> {
    let (input, _) = skip_comments_and_blank_lines(input)?;
    let (input, val) = script_value(input)?;
    let (input, _) = skip_comments_and_blank_lines(input)?;
    // dbg!(&val);

    Ok((input, val))
}

fn array<'a>(input: &'a str) -> IResult<&'a str, cw_model::Value, Error<'a>> {
    let (input, _) = context("opening_bracket", ws(char('{')))(input)?;
    let (input, values) = context("array_values", many1(array_value))(input)?;
    let (input, _) = context("closing_bracket", ws(char('}')))(input)?;

    Ok((
        input,
        cw_model::Value::StringArray(values.iter().map(|v| v.to_string()).collect()),
    ))
}

/// A number without a decimal
fn integer<'a>(input: &'a str) -> IResult<&'a str, i32, Error<'a>> {
    let (input, _) = opt(char('+'))(input)?;
    let (input, val) = character::complete::i32(input)?;
    let (input, _) = multispace1(input)?;
    return Ok((input, val));
}

fn integer_as_float<'a>(input: &'a str) -> IResult<&'a str, f32, Error<'a>> {
    map_res(integer, |v| Ok::<_, Error<'a>>(v as f32))(input)
}

/// A number with a decimal
fn decimal<'a>(input: &'a str) -> IResult<&'a str, f32, Error<'a>> {
    let (input, _) = opt(char('+'))(input)?;
    let (input, v) = map_res(
        recognize(tuple((opt(char('-')), digit1, char('.'), digit1))),
        |v: &str| v.parse::<f32>().map_err(|e| anyhow!(e)),
    )(input)?;
    let (input, _) = multispace1(input)?;
    Ok((input, v))
}

fn rgb<'a>(input: &'a str) -> IResult<&'a str, cw_model::Value, Error<'a>> {
    let (input, _) = ws(tag("rgb"))(input)?;
    let (input, _) = context("opening_bracket", ws(char('{')))(input)?;
    let (input, r) = context("r", integer)(input)?;
    let (input, g) = context("g", integer)(input)?;
    let (input, b) = context("b", integer)(input)?;
    let (input, a) = context("a", opt(integer))(input)?;
    let (input, _) = context("closing_bracket", ws(char('}')))(input)?;

    Ok((input, cw_model::Value::RGB(r, g, b, a)))
}

fn hsv<'a>(input: &'a str) -> IResult<&'a str, cw_model::Value, Error<'a>> {
    let (input, _) = tag("hsv")(input)?;
    let (input, _) = context("opening_bracket", ws(char('{')))(input)?;
    let (input, h) = context("h", alt((decimal, integer_as_float)))(input)?;
    let (input, s) = context("s", alt((decimal, integer_as_float)))(input)?;
    let (input, v) = context("v", alt((decimal, integer_as_float)))(input)?;
    let (input, a) = context("a", opt(alt((decimal, integer_as_float))))(input)?;
    let (input, _) = context("closing_bracket", ws(char('}')))(input)?;

    Ok((input, cw_model::Value::HSV(h, s, v, a)))
}

fn empty_entity<'a>(input: &'a str) -> IResult<&'a str, cw_model::Value, Error<'a>> {
    let (input, _) = context("opening_bracket", ws(char('{')))(input)?;
    let (input, _) = skip_comments_and_blank_lines(input)?;
    let (input, _) = context("closing_bracket", ws(char('}')))(input)?;

    Ok((
        input,
        cw_model::Value::Entity(cw_model::Entity {
            properties: HashMap::new(),
        }),
    ))
}

fn entity<'a>(input: &'a str) -> IResult<&'a str, cw_model::Value, Error<'a>> {
    let (input, _) = context("opening_bracket", ws(char('{')))(input)?;
    let (input, expressions) = context("expressions", many0(expression))(input)?;
    let (input, _) = context("closing_bracket", ws(char('}')))(input)?;

    let mut properties: HashMap<String, Vec<cw_model::PropertyInfo>> = HashMap::new();

    for expression in expressions {
        let items = properties.entry(expression.key.clone()).or_insert(vec![]);
        items.push(cw_model::PropertyInfo {
            value: expression.value,
            operator: expression.operator,
        });
    }

    Ok((
        input,
        cw_model::Value::Entity(cw_model::Entity { properties }),
    ))
}

fn eat_whitespace<'a>(input: &'a str) -> IResult<&'a str, (), Error<'a>> {
    let (input, _) = context("whitespace", multispace1)(input)?;
    Ok((input, ()))
}

fn skip_comments_and_blank_lines<'a>(input: &'a str) -> IResult<&'a str, (), Error<'a>> {
    let (input, _) = context(
        "skip_comments_and_blank_lines",
        many0(alt((comment, eat_whitespace))),
    )(input)?;
    Ok((input, ()))
}

fn define_identifier<'a>(input: &'a str) -> IResult<&'a str, &str, Error<'a>> {
    context("define", recognize(pair(char('@'), identifier)))(input)
}

fn define<'a>(input: &'a str) -> IResult<&'a str, Expression, Error<'a>> {
    let (input, _) = skip_comments_and_blank_lines(input)?;
    let (input, key) = define_identifier(input)?; // @identifier_name
    let (input, _) = ws(char('='))(input)?;
    let (input, value) = ws(script_value)(input).expect("failed to parse value!");
    let (input, _) = skip_comments_and_blank_lines(input)?;

    Ok((
        input,
        Expression {
            key: key.to_string(),
            operator: cw_model::Operator::Equals,
            value,
            is_define: true,
        },
    ))
}

pub fn parse_module<'a>(
    input: &'a str,
    type_path: &str,
    module_name: &str,
) -> IResult<&'a str, cw_model::Module, Error<'a>> {
    let (input, _) = skip_comments_and_blank_lines(input)?;
    let (input, expressions) = many0(alt((expression, define)))(input)?;

    let mut entities = HashMap::new();
    let mut defines = HashMap::new();
    let mut properties = HashMap::new();

    for expression in expressions {
        if expression.operator == cw_model::Operator::Equals {
            if expression.is_define {
                defines.insert(expression.key, expression.value);
            } else {
                if expression.value.is_entity() {
                    entities.insert(expression.key, expression.value);
                } else {
                    let items = properties.entry(expression.key.clone()).or_insert(vec![]);
                    items.push(cw_model::PropertyInfo {
                        value: expression.value,
                        operator: expression.operator,
                    });
                }
            }
        }
    }

    Ok((
        input,
        cw_model::Module {
            type_path: type_path.to_string(),
            filename: module_name.to_string(),
            entities,
            defines,
            properties,
        },
    ))
}

pub async fn parse_from_file(file_path: &str) -> Result<Module, anyhow::Error> {
    let path = PathBuf::from(file_path);
    let mut type_path = String::new();
    let mut cur_path = path.clone();

    while let Some(common_index) = cur_path
        .components()
        .position(|c| c.as_os_str() == "common")
    {
        if let Some(common_prefix) = cur_path
            .components()
            .take(common_index + 1)
            .collect::<PathBuf>()
            .to_str()
        {
            type_path = cur_path
                .strip_prefix(common_prefix)
                .unwrap()
                .parent()
                .unwrap()
                .to_string_lossy()
                .to_string();
            cur_path = cur_path.strip_prefix(common_prefix).unwrap().to_path_buf();
        }
    }

    type_path = ["common", &type_path]
        .iter()
        .collect::<PathBuf>()
        .to_slash_lossy()
        .to_string();

    let module_name = path.file_stem().unwrap().to_str().unwrap();
    let input = tokio::fs::read_to_string(file_path).await?;
    parse_module(&input, &type_path, module_name)
        .map(|(_, module)| module)
        .map_err(|e| anyhow!(e.to_string()))
}

impl Module {
    pub async fn parse_from_file(file_path: &str) -> Result<Module, anyhow::Error> {
        parse_from_file(file_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_from_file() {
        let input_file =
            "D:/SteamLibrary/steamapps/common/Stellaris/common/governments/civics/00_civics.txt";

        let module = parse_from_file(input_file).await.unwrap();

        assert_eq!(module.filename, "00_civics");
        assert_eq!(module.type_path, "common/governments/civics");
    }

    #[tokio::test]
    async fn test_eutab_edicts() {
        let input_file = "D:\\SteamLibrary\\steamapps\\workshop\\content\\281990\\804732593\\common\\edicts\\eutab_edicts.txt";

        let module = Module::parse_from_file(input_file).await.unwrap();

        assert_eq!(module.filename, "eutab_edicts");
        assert_eq!(module.type_path, "common/edicts");

        assert!(module.entities.len() > 0);

        assert!(module.entities.contains_key("e_eutab_defcon"));
        assert!(module.entities.contains_key("e_eutab_buy_influence"));
        assert!(module.entities.contains_key("e_eutab_bastion_dem"));
        assert!(module.entities.contains_key("e_eutab_rally_support"));
        assert!(module.entities.contains_key("e_eutab_emergency_powers"));
    }

    #[tokio::test]
    async fn test_eutab_edits_sm() {
        let input = r#"
        ##########################################################################
        # Empire edicts
        ##########################################################################
        
        @Edict0Cost = 75
        @Edict1Cost = 100
        @Edict2Cost = 200
        @Edict3Cost = 300
        
        @EdictDuration = 3600
        @FocusDuration = 1800
        
        @Focus1Cost = 80
        @Focus2Cost = 120
        
        @campaignCost = 1000
        @campaignDuration = 3600
        
        ##############################
        ##General
        ##############################
        
        e_eutab_defcon = {
            length = 720
            resources = {
                category = campaigns
                cost = {
                    energy = 500
                }
            }
        
            modifier = {
                ship_speed_mult = 0.35
                ship_winddown_mult = -0.5
                ship_ftl_jumpdrive_range_mult = 0.25
                ships_upkeep_mult = 0.15
                ship_emergency_ftl_mult = 0.5
            }
            
            ai_weight = {
                weight = 0	
            }
            
            prerequisites = {
                "tech_eutab_comp_defence_plan"
            }
        }
        
        "#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert!(module.entities.len() > 0);

        assert!(module.entities.contains_key("e_eutab_defcon"));
    }

    #[tokio::test]
    async fn scalar_value() {
        let input = r#"
        #Innate Command
        tech_eutab_innate_command = {
            cost = @tier4cost3
            area = society
            tier = 4
            category = {
                military_theory
            }
            is_custom_tech_1 = yes
            prerequisites = {
                "tech_eutab_wargame"
            }
            weight = @tier4weight3
            potential = {
                is_militarist = yes
            }
            gateway = eu_vision
            modifier = {
                country_edict_fund_add = 25
                leader_admirals_cost_mult = -0.33
                leader_generals_cost_mult = -0.33
            }
            weight_modifier = {
                modifier = {
                    factor = 0
                    NOT = {
                        has_ascension_perk = ap_one_vision
                    }
                }
                modifier = {
                    factor = value:tech_weight_likelihood
                    research_leader = {
                        area = society
                        has_trait = "leader_trait_expertise_military_theory"
                    }
                }
            }
            ai_weight = {
                modifier = {
                    factor = value:tech_weight_likelihood
                    research_leader = {
                        area = society
                        has_trait = "leader_trait_expertise_military_theory"
                    }
                }
            }
        }
        "#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert!(module.entities.len() > 0);

        assert!(module.entities.contains_key("tech_eutab_innate_command"));
    }

    #[tokio::test]
    async fn eutab_events() {
        let input = r#"
namespace = eutab

#machine mod
country_event = {
	id = eutab.8
	title = "eutab.8.name"
	desc = "eutab.8.desc"
	picture = GFX_evt_machine_sapience
	#picture = GFX_evt_eutab_machine_mod
	show_sound = event_evolution_mastery
	
	is_triggered_only = yes

	option = {
		name = Acknowledged
	}
}
"#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert!(module.entities.len() == 1);

        assert!(module.entities.contains_key("country_event"));

        let event = module.entities.get("country_event").unwrap().entity();

        assert_eq!(
            event.properties.get("id").unwrap()[0].value.string(),
            "eutab.8"
        );

        assert_eq!(
            event.properties.get("title").unwrap()[0].value.string(),
            "eutab.8.name"
        );

        assert_eq!(
            event.properties.get("desc").unwrap()[0].value.string(),
            "eutab.8.desc"
        );

        assert_eq!(
            event.properties.get("picture").unwrap()[0].value.string(),
            "GFX_evt_machine_sapience"
        );

        assert_eq!(
            event.properties.get("show_sound").unwrap()[0]
                .value
                .string(),
            "event_evolution_mastery"
        );

        assert_eq!(
            event.properties.get("is_triggered_only").unwrap()[0]
                .value
                .string(),
            "yes"
        );

        assert!(event.properties.get("option").unwrap()[0].value.is_entity());
    }

    #[tokio::test]
    async fn achievement_patron() {
        let input = r#"
        achievement_patron = {
            id = 33
        
            possible = {
                is_ironman = yes
                has_leviathans = yes
            }
        
            happened = {
                has_country_flag = 10yr_patronage
            }
        }"#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert!(module.entities.len() == 1);
    }

    #[tokio::test]
    async fn preset_scion() {
        let input = r#"
        preset_scion = {
            icon = "GFX_diplomacy_status_is_scion"
            term_data = {
                discrete_terms = {
                    {
                        key = specialist_type
                        value = specialist_none
                    }
                    {
                        key = subject_integration
                        value = subject_can_not_be_integrated
                    }
                    {
                        key = subject_diplomacy
                        value = subject_can_do_diplomacy
                    }
                    {
                        key = subject_expand
                        value = subject_can_expand
                    }
                    {
                        key = joins_overlord_wars
                        value = joins_overlord_wars_all
                    }
                    {
                        key = joins_subject_wars
                        value = joins_subject_wars_none
                    }
                    {
                        key = subject_holdings_limit
                        value = subject_holdings_limit_0
                    }
                    {
                        key = has_access
                        value = no_access
                    }
                    {
                        key = subject_sensors
                        value = subject_does_not_get_sensors
                    }
                    {
                        key = subject_loyalty
                        value = subject_loyalty_effects
                    }
                    {
                        key = protectorate
                        value = subject_is_not_protectorate
                    }
                }
            }
            potential = {
                any_agreement = {
                    agreement_preset = preset_scion
                }
                from = {
                    OR = {
                        is_country_type = fallen_empire
                        is_country_type = awakened_fallen_empire
                    }
                }
            }
        } 
        "#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert!(module.entities.len() == 1);
    }

    #[tokio::test]
    async fn weird_comments() {
        let input = r#"
        dragon_dummy = { #This country type is only needed for the origin_here_be_dragons first contact story. It should never be used for anything else.
            government = no
            observable = no
            share_communications = no
            ai = {
                enabled = no
            }
            faction = {
                needs_border_access = no
                generate_borders = no
                needs_colony = no
                auto_delete = no
            }
            modules = {
                exclusive_diplomacy_module = { # Nothing is allowed, but we need this for relationships.
                }
                basic_technology_module = {}
            }
        }
        "#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert!(module.entities.len() == 1);
    }

    #[tokio::test]
    async fn one_line() {
        let input = r#"
        crisis_level_2 = {
            # REQUIREMENTS
            allow = {
                custom_tooltip = {
                    success_text =	crisis_level_2_req_clear_tooltip
                    fail_text =	crisis_level_2_req_tooltip
                    has_country_flag = crisis_special_project_1_complete
                }
                hidden_trigger = {
                    NOT = {
                        has_active_event = {
                            crisis.4120
                        }
                    }
                }
            }
            required_menace = 1000
        
            # REWARDS
            perks = {
                menp_crisis_corvette
                menp_base_breaker
                menp_relentless_aggression
            }
        
            on_unlock = {
                hidden_effect = {
                    owner = {
                        add_event_chain_counter = {
                            event_chain = "become_the_crisis_chain"
                            counter = "crisis_level_reached"
                            amount = 1
                        }
                        country_event = { id = crisis.4145 days = 3 }
                    }
                }
            }
        }
        "#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert!(module.entities.len() == 1);
    }

    #[tokio::test]
    async fn level_5() {
        let input = r#"
        crisis_level_5 = {
            # REQUIREMENTS
            allow = {
                custom_tooltip = {
                    success_text =	crisis_level_5_req_clear_tooltip
                    fail_text =	crisis_level_5_req_tooltip
                    has_country_flag = crisis_special_project_4_complete
                }
                hidden_trigger = {
                    NOT = {
                        has_active_event = {
                            crisis.4135
                        }
                    }
                }
                is_subject = no
            }
            required_menace = 10000
        
            # REWARDS
            perks = {
                menp_megastructure
                menp_star_eater
                menp_mobile_bulwark
                menp_paid_in_ambition
            }
        
            on_unlock = {
                owner = {
                    if = {
                        limit = { has_federation = yes }
                        leave_alliance = { override_requirements = yes }
                    }
                    tooltip = {
                        if = {
                            limit = {
                                is_galactic_community_formed = yes
                                is_galactic_community_member = yes
                            }
                            remove_from_galactic_community = yes
                        }
                    }
                }
                hidden_effect = {
                    # force Declare Crisis resolution through an appropriate party
                    if = {
                        limit = { any_playable_country = { is_galactic_custodian = yes } }
                        random_playable_country = {
                            limit = { is_galactic_custodian = yes }
                            pass_targeted_resolution = {
                                resolution = resolution_declare_crisis
                                target = root.owner
                            }
                        }
                    }
                    else_if = {
                        limit = { any_playable_country = { is_galactic_emperor = yes } }
                        random_playable_country = {
                            limit = { is_galactic_emperor = yes }
                            pass_targeted_resolution = {
                                resolution = resolution_declare_crisis_empire
                                target = root.owner
                            }
                        }
                    }
                    else_if = {
                        limit = { owner = { any_neighbor_country = { is_galactic_community_member = yes } } }
                        owner = {
                            random_neighbor_country = {
                                limit = { is_galactic_community_member = yes }
                                pass_targeted_resolution = {
                                    resolution = resolution_declare_crisis
                                    target = root.owner
                                }
                            }
                        }
                    }
                    else_if = {
                        limit = { any_playable_country = { is_galactic_community_member = yes } }
                        random_playable_country = {
                            limit = { is_galactic_community_member = yes }
                            pass_targeted_resolution = {
                                resolution = resolution_declare_crisis
                                target = root.owner
                            }
                        }
                    }
                    owner = {
                        room_name_override = ""
                        add_event_chain_counter = {
                            event_chain = "become_the_crisis_chain"
                            counter = "crisis_level_reached"
                            amount = 1
                        }
                        end_event_chain = become_the_crisis_chain
                        country_event = { id = crisis.6000 days = 5 }
                    }
                }
            }
        }
        "#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert_eq!(module.entities.len(), 1);
    }

    #[tokio::test]
    async fn rgb_array() {
        let input = r#"
        fallen_empire = {
            rgb { 0 0 255 }
            rgb { 0 255 0 }
            rgb { 255 20 150 }
            rgb { 125 38 205 }
            rgb { 30 144 255 }
            rgb { 0 206 209 }
            rgb { 0 139 69 }
            rgb { 192 255 62 }
            rgb { 255 255 0 }
            rgb { 255 165 0 }
            hsv { 0 100 0.8 }
        }"#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert_eq!(module.properties.len(), 1);
    }

    #[tokio::test]
    async fn array_with_comments() {
        let input = r#"
        SCIENCE_SHIP_ANOMALY_RESEARCH_DAYS = { # equal level is middle (count of items / 2)
			20		#9 levels above anomaly
			24		#8 levels above anomaly
			30		#7 levels above anomaly
			35		#6 levels above anomaly
			45		#5 levels above anomaly
			55		#4 levels above anomaly
			65		#3 levels above anomaly
			80		#2 levels above anomaly
			100		#1 level above anomaly
			120		#0 level equal to anomaly
			180		#1 level below anomaly
			300		#2 levels below anomaly
			540		#3 levels below anomaly
			720		#4 levels below anomaly
			1080	#5 levels below anomaly
			1440	#6 levels below anomaly
			2160	#7 levels below anomaly
			2880	#8 levels below anomaly
			5760	#9 levels below anomaly
		}
        "#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert_eq!(module.properties.len(), 1);
    }

    #[tokio::test]
    async fn map_modes() {
        let input = r#"
        default_map_mode = {
            icon = "GFX_map_mode_default"
            enable_terra_incognita = yes
            can_change_point_of_view = no
            shortcut = "CTRL+z"
        
        
            # Color Galactic Empire members red depending on imperial authority
            # See GALACTIC_EMPIRE_BASE_AUTHORITY and GALACTIC_EMPIRE_MAX_AUTHORITY in defines
        
            color = {
                type = country_and_borders
                zoom = 1300
                value = rgb { 235 0 18 0 }
                condition = { is_galactic_emperor = yes }
                hardcoded_tooltip = country
            }
        
            color = {
                type = country_and_borders
                zoom = 1300
                filter = unions
                value = rgb { 235 0 18 0 }
                condition = {
                    has_galactic_emperor = yes
                    imperial_authority >= 100
                    is_part_of_galactic_council = yes
                }
                hardcoded_tooltip = country
            }
        
            color = {
                type = borders
                filter = unions
                zoom = 1300
                value = rgb { 235 0 18 0 }
                condition = {
                    has_galactic_emperor = yes
                    imperial_authority >= 150
                    is_galactic_community_member = yes
                }
                hardcoded_tooltip = country
            }
        
            color = {
                type = borders
                zoom = 1300
                value = rgb { 235 0 18 0 }
                condition = {
                    has_galactic_emperor = yes
                    imperial_authority >= 100
                    is_part_of_galactic_council = yes
                }
                hardcoded_tooltip = country
            }
        
            color = {
                type = borders
                zoom = 1300
                value = rgb { 235 0 18 0 }
                condition = {
                    has_galactic_emperor = yes
                    imperial_authority >= 150
                    is_galactic_community_member = yes
                }
                hardcoded_tooltip = country
            }
        
            color = {
                value = country
                condition = { always = yes }
                hardcoded_tooltip = country
            }
        }
        "#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert_eq!(module.entities.len(), 1);
    }

    #[tokio::test]
    async fn number_maybe() {
        let input = r#"
        demand = {
            title = "PROSPERITY_MILITARY_APPLICATIONS"
            unfulfilled_title = "PROSPERITY_NO_MILITARY_APPLICATIONS"
            desc = "PROSPERITY_MILITARY_APPLICATIONS_DESC"
    
            unfulfilled_effect = -5
            fulfilled_effect = +0.001
    
            potential = {
                exists = owner
                owner = {
                    host_has_dlc = "Ancient Relics Story Pack"
                    has_technology = tech_arcane_deciphering
                    OR = {
                        has_modifier = artifact_find_military_application_army
                        has_modifier = artifact_find_military_application_shield_damage
                        has_modifier = artifact_find_military_application_armor_damage
                    }
                }
            }
    
            trigger = {
                owner = {
                    NOR = {
                        has_modifier = artifact_find_military_application_army
                        has_modifier = artifact_find_military_application_shield_damage
                        has_modifier = artifact_find_military_application_armor_damage
                    }
                }
            }
        }
        "#;

        let module = parse_module(input, "common/edicts", "eutab_edicts")
            .unwrap()
            .1;

        assert_eq!(module.entities.len(), 1);
    }
}
