use anyhow::{anyhow, Result};
#[cfg(test)]
use mockall::automock;
use std::collections::HashMap;

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
    All(Box<PrintableTerrain>),
    HashMap(Option<HashMap<String, String>>),
    Commands(Option<Commands>),
}

#[cfg_attr(test, automock)]
pub mod print {
    use anyhow::{Context, Result};
    use std::collections::HashMap;

    use super::{
        parse_template, Data, ALIASES, ALIAS_TEMPLATE, CONSTRUCTORS, CONSTRUCTORS_TEMPLATE,
        DESTRUCTORS, DESTRUCTORS_TEMPLATE, ENV, ENV_TEMPLATE, MAIN, MAIN_TEMPLATE,
    };
    use crate::types::{commands::Commands, get::PrintableTerrain};

    pub fn all(terrain: PrintableTerrain) -> Result<()> {
        let text = parse_template(
            vec![
                (ENV, ENV_TEMPLATE),
                (MAIN, MAIN_TEMPLATE),
                (ALIASES, ALIAS_TEMPLATE),
                (CONSTRUCTORS, CONSTRUCTORS_TEMPLATE),
                (DESTRUCTORS, DESTRUCTORS_TEMPLATE),
            ],
            MAIN,
            Data::All(Box::new(terrain)),
        )
        .context("failed to render terrain output")?;
        println!("{}", text);
        Ok(())
    }

    pub fn env(env: Option<HashMap<String, String>>) -> Result<()> {
        let text = parse_template(vec![(ENV, ENV_TEMPLATE)], ENV, Data::HashMap(env))?;
        println!("{}", text);
        Ok(())
    }

    pub fn aliases(aliases: Option<HashMap<String, String>>) -> Result<()> {
        let text = parse_template(
            vec![(ALIASES, ALIAS_TEMPLATE)],
            ALIASES,
            Data::HashMap(aliases),
        )?;
        println!("{}", text);
        Ok(())
    }

    pub fn constructors(constructors: Option<Commands>) -> Result<()> {
        let text = parse_template(
            vec![(CONSTRUCTORS, CONSTRUCTORS_TEMPLATE)],
            CONSTRUCTORS,
            Data::Commands(constructors),
        )?;
        println!("{}", text);
        Ok(())
    }

    pub fn destructors(destructors: Option<Commands>) -> Result<()> {
        let text = parse_template(
            vec![(DESTRUCTORS, DESTRUCTORS_TEMPLATE)],
            DESTRUCTORS,
            Data::Commands(destructors),
        )?;
        println!("{}", text);
        Ok(())
    }
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
        .map(|(name, template)| handlebar.register_template_string(name, template))
        .collect();

    if let Err(e) = errors {
        return Err(anyhow!("Unable to register templates {}", e));
    }

    match data {
        Data::All(data) => Ok(handlebar.render(template_to_parse, &data)?),
        Data::HashMap(data) => Ok(handlebar.render(template_to_parse, &data)?),
        Data::Commands(data) => Ok(handlebar.render(template_to_parse, &data)?),
    }
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
        echo entering terrain \n        echo entering biome 'example_biome' \nDestructors:
    foreground:
        echo exiting terrain \n        echo exiting biome 'example_biome' \n"
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
            Data::All(Box::new(terrain)),
        )?;

        assert_eq!(expected, actual);

        Ok(())
    }
}
