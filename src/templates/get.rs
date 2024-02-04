use std::collections::HashMap;

use anyhow::{anyhow, Result};

use crate::types::{commands::Commands, get::PrintableTerrain};

const MAIN_TEMPLATE: &str = include_str!("../../templates/get.hbs");
const ENV_TEMPLATE: &str = include_str!("../../templates/get_env.hbs");
const ALIAS_TEMPLATE: &str = include_str!("../../templates/get_aliases.hbs");
const CONSTRUCTORS_TEMPLATE: &str = include_str!("../../templates/get_constructors.hbs");
const DESTRUCTORS_TEMPLATE: &str = include_str!("../../templates/get_destructors.hbs");

const ENV: &str = "env";
const MAIN: &str = "MAIN";
const ALIASES: &str = "aliases";
const CONSTRUCTORS: &str = "constructors";
const DESTRUCTORS: &str = "destructors";

enum Data {
    All(PrintableTerrain),
    HashMap(Option<HashMap<String, String>>),
    Commands(Option<Commands>),
}

fn parse_template(
    template_strings: Vec<(&str, &str)>,
    template_to_parse: &str,
    data: Data,
) -> Result<String> {
    let mut handlebar = handlebars::Handlebars::new();
    handlebar.set_strict_mode(true);
    let errors: Result<Vec<_>, _> = template_strings
        .iter()
        .map(|(name, template)| {
            return handlebar.register_template_string(name, template);
        })
        .collect();

    if let Err(e) = errors {
        return Err(anyhow!("Unable to register templates {}", e));
    }

    match data {
        Data::All(data) => return Ok(handlebar.render(template_to_parse, &data)?),
        Data::HashMap(data) => return Ok(handlebar.render(template_to_parse, &data)?),
        Data::Commands(data) => return Ok(handlebar.render(template_to_parse, &data)?),
    }
}

pub fn print_all(terrain: PrintableTerrain) -> Result<()> {
    let text = parse_template(
        vec![
            (ENV, ENV_TEMPLATE),
            (MAIN, MAIN_TEMPLATE),
            (ALIASES, ALIAS_TEMPLATE),
            (CONSTRUCTORS, CONSTRUCTORS_TEMPLATE),
            (DESTRUCTORS, DESTRUCTORS_TEMPLATE),
        ],
        MAIN,
        Data::All(terrain),
    )?;
    println!("{}", text);
    return Ok(());
}

pub fn print_env(env: Option<HashMap<String, String>>) -> Result<()> {
    let text = parse_template(vec![(ENV, ENV_TEMPLATE)], ENV, Data::HashMap(env))?;
    println!("{}", text);
    return Ok(());
}

pub fn print_aliases(aliases: Option<HashMap<String, String>>) -> Result<()> {
    let text = parse_template(vec![(ALIASES, ALIAS_TEMPLATE)], ALIASES, Data::HashMap(aliases))?;
    println!("{}", text);
    return Ok(());
}

pub fn print_constructors(constructors: Option<Commands>) -> Result<()> {
    let text = parse_template(
        vec![(CONSTRUCTORS, CONSTRUCTORS_TEMPLATE)],
        CONSTRUCTORS,
        Data::Commands(constructors),
    )?;
    println!("{}", text);
    return Ok(());
}

pub fn print_destructors(destructors: Option<Commands>) -> Result<()> {
    let text = parse_template(
        vec![(DESTRUCTORS, DESTRUCTORS_TEMPLATE)],
        DESTRUCTORS,
        Data::Commands(destructors),
    )?;
    println!("{}", text);
    return Ok(());
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use crate::types::{args::BiomeArg, terrain::Terrain};

    use super::{
        parse_template, Data, ALIASES, ALIAS_TEMPLATE, CONSTRUCTORS, CONSTRUCTORS_TEMPLATE,
        DESTRUCTORS, DESTRUCTORS_TEMPLATE, ENV, ENV_TEMPLATE, MAIN, MAIN_TEMPLATE,
    };

    #[test]
    fn test_printable_terrain() -> Result<()> {
        let terrain = Terrain::default();
        let biome_arg = BiomeArg::Default;

        let expected = "Default Biome: example_biome          Selected Biome: example_biome
Environment Variables:
    EDITOR=\"vim\"
Aliases:
    tedit=\"terrainium edit\"
    tenter=\"terrainium enter\"
Constructors:
    foreground:
        echo entering terrain
        echo entering biome 'example_biome'
Destructors:
    foreground:
        echo exiting terrain
        echo exiting biome 'example_biome'
"
        .to_string();

        let mut terrain = terrain.get_printable_terrain(Some(biome_arg))?;
        terrain.all = true;
        let actual = parse_template(
            vec![
                (ENV, ENV_TEMPLATE),
                (MAIN, MAIN_TEMPLATE),
                (ALIASES, ALIAS_TEMPLATE),
                (CONSTRUCTORS, CONSTRUCTORS_TEMPLATE),
                (DESTRUCTORS, DESTRUCTORS_TEMPLATE),
            ],
            MAIN,
            Data::All(terrain),
        )?;

        assert_eq!(expected, actual);

        return Ok(());
    }
}
