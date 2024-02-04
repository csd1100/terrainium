use std::path::PathBuf;

use anyhow::Result;

const MAIN_TEMPLATE: &str = include_str!("../../templates/zsh_final_script.hbs");
const ALIAS_TEMPLATE: &str = include_str!("../../templates/zsh_aliases.hbs");
const CONSTRUCTORS_TEMPLATE: &str = include_str!("../../templates/zsh_constructors.hbs");
const DESTRUCTORS_TEMPLATE: &str = include_str!("../../templates/zsh_destructors.hbs");

pub fn generate_zsh_script(_path: PathBuf) -> Result<()> {
    let mut handlebar = handlebars::Handlebars::new();
    handlebar.set_strict_mode(true);
    handlebar.register_template_string("main", MAIN_TEMPLATE)?;
    handlebar.register_template_string("aliases", ALIAS_TEMPLATE)?;
    handlebar.register_template_string("constructors", CONSTRUCTORS_TEMPLATE)?;
    handlebar.register_template_string("destructors", DESTRUCTORS_TEMPLATE)?;
    todo!();

    return Ok(());
}

pub fn compile(_path: PathBuf) -> Result<()> {
    return Ok(());
}
