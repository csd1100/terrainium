use crate::client::types::environment::render;
use crate::common::constants::{
    CONSTRUCTORS, FS_BIOME_NAME, FS_COMMAND, FS_CONSTRUCTORS, FS_DESTRUCTORS, FS_END_TIME,
    FS_LOG_PATH, FS_SESSION_ID, FS_START_TIME, FS_STATUS, FS_TERRAIN_NAME, FS_TOML_PATH,
    STATUS_MAIN_TEMPLATE_NAME,
};
use crate::common::types::terrain_state::{CommandState, ExecutionContext, TerrainState};
use owo_colors::{OwoColorize, Style};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

fn styles() -> HashMap<String, (Style, Vec<Style>)> {
    let mut styles = HashMap::<String, (Style, Vec<Style>)>::new();
    let vec = vec![Style::new(), Style::new()];
    styles.insert(FS_SESSION_ID.to_string(), (Style::new(), vec.clone()));
    styles.insert(FS_TERRAIN_NAME.to_string(), (Style::new(), vec.clone()));
    styles.insert(FS_BIOME_NAME.to_string(), (Style::new(), vec.clone()));
    styles.insert(FS_TOML_PATH.to_string(), (Style::new(), vec.clone()));
    styles.insert(FS_START_TIME.to_string(), (Style::new(), vec.clone()));
    styles.insert(FS_END_TIME.to_string(), (Style::new(), vec.clone()));
    styles.insert(FS_CONSTRUCTORS.to_string(), (Style::new(), vec.clone()));
    styles.insert(FS_DESTRUCTORS.to_string(), (Style::new(), vec.clone()));
    styles.insert(FS_COMMAND.to_string(), (Style::new(), vec.clone()));
    styles.insert(FS_STATUS.to_string(), (Style::new(), vec.clone()));
    styles.insert(FS_LOG_PATH.to_string(), (Style::new(), vec));
    styles
}

#[derive(Serialize)]
struct StyledPair<T> {
    field: String,
    value: T,
}

#[derive(Serialize)]
pub(in crate::common::types) struct StyledTerrainState {
    session_id: StyledPair<String>,
    terrain_name: StyledPair<String>,
    biome_name: StyledPair<String>,
    toml_path: StyledPair<String>,
    start_timestamp: StyledPair<String>,
    end_timestamp: StyledPair<String>,
    execute_context: StyledExecutionContext,
}

const STATUS_MAIN_TEMPLATE: &str = include_str!("../../../templates/status.hbs");
impl StyledTerrainState {
    pub(in crate::common::types) fn render(self) -> String {
        let mut templates = BTreeMap::<String, String>::new();
        templates.insert(
            STATUS_MAIN_TEMPLATE_NAME.to_string(),
            STATUS_MAIN_TEMPLATE.to_string(),
        );
        render(STATUS_MAIN_TEMPLATE_NAME.to_string(), templates, self).unwrap()
    }
}

#[derive(Serialize)]
struct StyledExecutionContext {
    constructors_state: StyledPair<Vec<StyledCommandState>>,
    destructors_state: StyledPair<Vec<StyledCommandState>>,
}

#[derive(Serialize)]
struct StyledCommandState {
    command: StyledCommandToRun,
    log_path: StyledPair<String>,
    status: StyledPair<String>,
}

#[derive(Serialize)]
struct StyledCommandToRun {
    exe: String,
    args: String,
}

impl From<TerrainState> for StyledTerrainState {
    fn from(value: TerrainState) -> Self {
        let session_id: StyledPair<String> = style_applied_string(FS_SESSION_ID, value.session_id);
        let terrain_name: StyledPair<String> =
            style_applied_string(FS_TERRAIN_NAME, value.terrain_name);
        let biome_name: StyledPair<String> = style_applied_string(FS_BIOME_NAME, value.biome_name);
        let toml_path: StyledPair<String> = style_applied_string(FS_TOML_PATH, value.toml_path);
        let start_timestamp: StyledPair<String> =
            style_applied_string(FS_START_TIME, value.start_timestamp);
        let end_timestamp: StyledPair<String> =
            style_applied_string(FS_END_TIME, value.end_timestamp);
        let execute_context: StyledExecutionContext = value.execute_context.into();

        StyledTerrainState {
            session_id,
            terrain_name,
            biome_name,
            toml_path,
            start_timestamp,
            end_timestamp,
            execute_context,
        }
    }
}

impl From<ExecutionContext> for StyledExecutionContext {
    fn from(value: ExecutionContext) -> Self {
        let constructors: Vec<StyledCommandState> = value
            .constructors_state
            .into_iter()
            .map(Into::into)
            .collect();

        let destructors: Vec<StyledCommandState> = value
            .destructors_state
            .into_iter()
            .map(Into::into)
            .collect();

        let constructor_style = styles().get(FS_CONSTRUCTORS).expect("").clone();
        let destructor_style = styles().get(FS_DESTRUCTORS).expect("").clone();

        let constructors_state: StyledPair<Vec<StyledCommandState>> = StyledPair {
            field: FS_CONSTRUCTORS.style(constructor_style.0).to_string(),
            value: constructors,
        };

        let destructors_state: StyledPair<Vec<StyledCommandState>> = StyledPair {
            field: FS_DESTRUCTORS.style(destructor_style.0).to_string(),
            value: destructors,
        };
        StyledExecutionContext {
            constructors_state,
            destructors_state,
        }
    }
}

impl From<CommandState> for StyledCommandState {
    fn from(value: CommandState) -> Self {
        let log_path: StyledPair<String> = style_applied_string(FS_LOG_PATH, value.log_path);
        let status: StyledPair<String> = style_applied_string(FS_STATUS, value.status.into());

        let field = if value.operation == CONSTRUCTORS {
            FS_CONSTRUCTORS
        } else {
            FS_DESTRUCTORS
        };

        let mut styles = styles().get(field).unwrap().clone();
        let exe_style = styles.1.pop().unwrap();
        let args = styles.1.pop().unwrap();

        let args: String = value
            .command
            .args()
            .iter()
            .map(|x| x.style(args).to_string())
            .collect();

        let command: StyledCommandToRun = StyledCommandToRun {
            exe: value.command.exe().style(exe_style).to_string(),
            args,
        };

        StyledCommandState {
            command,
            log_path,
            status,
        }
    }
}

fn style_applied_string(field: &'static str, value: String) -> StyledPair<String> {
    let mut styles = styles().get(field).expect("").clone();
    StyledPair {
        field: field.style(styles.0).to_string(),
        value: value.style(styles.1.remove(0)).to_string(),
    }
}
