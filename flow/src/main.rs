mod output;

use clap::{Parser, Subcommand};
use flow::{
    Autonomy, ContextUpdate, Definition, DefinitionName, Instance, InstanceName, InstanceSavePolicy, Move,
    Scope, StateName, Storage,
};
use rust_utils::raise::RaiseExt;
use uuid::Uuid;

const DEFINITION_TEMPLATE: &str = include_str!("../templates/definition.toml");
const HELP_OVERVIEW: &str = include_str!("../help.txt");

#[derive(Parser)]
#[command(about = HELP_OVERVIEW)]
enum Cli {
    /// Start a new instance from a definition.
    Start {
        /// Definition to start from.
        definition_name: DefinitionName,
        /// Optional instance name; generated if omitted.
        instance_name: Option<InstanceName>,
        /// Save globally, requiring an existing global definition.
        #[arg(short, long)]
        global: bool,
        /// Save globally, copying the local definition if absent globally.
        #[arg(short = 'G', long = "copy-definition")]
        copy_definition: bool,
    },

    /// Move to a listed next state.
    Next {
        /// Instance to move.
        instance_name: InstanceName,
        /// Listed next state.
        target: String,
    },

    /// Jump to any defined state, including exceptional moves.
    Jump {
        /// Instance to move.
        instance_name: InstanceName,
        /// Destination state.
        target: String,
    },

    /// Show an instance's guidance or list all instances.
    Status {
        /// Instance to inspect; omit to list all instances.
        instance_name: Option<InstanceName>,
    },

    /// List definitions or states in a definition.
    List {
        #[command(subcommand)]
        target: ListTarget,
    },

    /// Set or remove persistent instance context.
    Context {
        #[command(subcommand)]
        action: ContextAction,
    },

    /// Set autonomy for a state or a range along next paths.
    Autonomy {
        #[command(subcommand)]
        action: AutonomyAction,
    },

    /// Create a definition from the bundled template.
    NewDefinitionFromTemplate {
        /// Name for the new definition.
        definition_name: DefinitionName,
        #[arg(short, long)]
        global: bool,
    },
}

#[derive(Subcommand)]
enum ListTarget {
    /// List available definitions.
    Definitions,
    /// List states in definition order.
    States { definition_name: DefinitionName },
}

#[derive(Subcommand)]
enum ContextAction {
    /// Store or replace a context value.
    Set {
        instance_name: InstanceName,
        key: String,
        value: String,
    },
    /// Remove a context value.
    Remove {
        instance_name: InstanceName,
        key: String,
    },
}

#[derive(Subcommand)]
enum AutonomyAction {
    /// Set a state's level or every state on next paths through an end state.
    Set {
        /// Instance to configure.
        instance_name: InstanceName,
        /// guided, steered, or autonomous.
        level: Autonomy,
        /// First state to configure.
        state: String,
        /// Inclusive end state; omit to change only the first state.
        end_state: Option<String>,
    },
}

#[rust_utils::raise_handler]
fn main() {
    run(Cli::parse());
}

fn run(cli: Cli) {
    match cli {
        Cli::Start { definition_name, instance_name, global, copy_definition } => {
            start(definition_name, instance_name, global, copy_definition)
        }
        Cli::NewDefinitionFromTemplate { definition_name, global } => {
            create_definition_from_template(definition_name, global)
        }
        Cli::Status { instance_name: Some(instance_name) } => status_instance(instance_name),
        Cli::Status { instance_name: None } => status_all(),
        Cli::List { target } => list(target),
        Cli::Context { action } => change_context(action),
        Cli::Autonomy { action } => change_autonomy(action),
        Cli::Next { instance_name, target } => move_instance(instance_name, Move::Next(StateName(target))),
        Cli::Jump { instance_name, target } => move_instance(instance_name, Move::JumpTo(StateName(target))),
    }
}

fn start(
    definition_name: DefinitionName,
    instance_name: Option<InstanceName>,
    global: bool,
    copy_definition: bool,
) {
    let name = instance_name.unwrap_or_else(|| {
        InstanceName::parse(format!("{}-{}", definition_name, Uuid::new_v4().simple())).raise()
    });

    let definition = Definition::load_from_name(definition_name.as_str());
    let instance = definition.into_new_instance(name);
    let path = instance.save_new(InstanceSavePolicy::from_cli_args(global, copy_definition));
    println!("{}", output::created_instance(&instance, &path));
}

fn create_definition_from_template(definition_name: DefinitionName, global: bool) {
    let scope = if global { Scope::Global } else { Scope::Local };
    let path = Definition::create_from_template(definition_name.clone(), DEFINITION_TEMPLATE, scope);
    println!("{}", output::created_definition(&definition_name, &path));
}

fn status_instance(instance_name: InstanceName) {
    let instance = Instance::load_from_name(instance_name.as_str());
    print!("{}", output::InstanceStatus(&instance));
}

fn status_all() {
    let instances = Instance::load_all();
    print!("{}", output::InstancesTable(&instances));
}

fn list(target: ListTarget) {
    match target {
        ListTarget::Definitions => {
            let names = Storage::current().definition_names();
            let definitions: Vec<_> = names
                .iter()
                .map(|name| Definition::load_from_name(name.as_str()))
                .collect();
            print!("{}", output::DefinitionsTable(&definitions));
        }
        ListTarget::States { definition_name } => {
            let definition = Definition::load_from_name(definition_name.as_str());
            print!("{}", output::StatesTable(&definition));
        }
    }
}

fn change_context(action: ContextAction) {
    let (name, change) = match action {
        ContextAction::Set { instance_name, key, value } => {
            (instance_name, ContextUpdate::Set { key, value })
        }
        ContextAction::Remove { instance_name, key } => (instance_name, ContextUpdate::Remove { key }),
    };
    Instance::load_from_name(name.as_str()).update_context(change);
}

fn change_autonomy(action: AutonomyAction) {
    match action {
        AutonomyAction::Set { instance_name, level, state, end_state } => {
            let changed = Instance::load_from_name(instance_name.as_str()).set_autonomy(
                level,
                &state,
                end_state.as_deref(),
            );
            print!("{}", output::autonomy_changed(level, &changed));
        }
    }
}

fn move_instance(instance_name: InstanceName, movement: Move) {
    let instance = Instance::load_from_name(instance_name.as_str()).move_to(movement);
    println!("{}", output::moved_instance(&instance));
}
