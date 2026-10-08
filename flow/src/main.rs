use clap::Parser;
use flow::{Definition, DefinitionName, Instance, InstanceName, InstanceSavePolicy, Move, Scope, StateName};
use rust_utils::raise::RaiseExt;
use uuid::Uuid;

const DEFINITION_TEMPLATE: &str = include_str!("../templates/definition.toml");

#[derive(Parser)]
enum Cli {
    Start {
        definition_name: DefinitionName,
        instance_name: Option<InstanceName>,
        /// Save globally, requiring an existing global definition.
        #[arg(short, long)]
        global: bool,
        /// Save globally, copying the local definition if absent globally.
        #[arg(short = 'G', long = "copy-definition")]
        copy_definition: bool,
    },

    Next {
        instance_name: InstanceName,
        target: String,
    },

    Jump {
        instance_name: InstanceName,
        target: String,
    },

    Status {
        instance_name: Option<InstanceName>,
    },

    NewDefinitionFromTemplate {
        definition_name: DefinitionName,
        #[arg(short, long)]
        global: bool,
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
    println!("created instance {} at {}", instance.name(), path.display());
}

fn create_definition_from_template(definition_name: DefinitionName, global: bool) {
    let scope = if global { Scope::Global } else { Scope::Local };
    let path = Definition::create_from_template(definition_name.clone(), DEFINITION_TEMPLATE, scope);
    println!("created definition {} at {}", definition_name, path.display());
}

fn status_instance(instance_name: InstanceName) {
    let instance = Instance::load_from_name(instance_name.as_str());
    let definition = instance.definition();
    let state = definition
        .states()
        .iter()
        .find(|state| state.name == instance.state().0)
        .expect("the instance state was validated when it was loaded");

    println!(
        "instance: {}\ndefinition: {}\nstate: {}",
        instance.name(),
        definition.name(),
        instance.state().0
    );
    print_optional("definition summary", definition.summary().as_deref());
    print_optional("definition details", definition.details().as_deref());
    print_steps("definition steps", definition.steps());
    print_optional("state summary", state.summary.as_deref());
    print_optional("state details", state.details.as_deref());
    print_steps("state steps", &state.steps);

    if !state.next.is_empty() {
        println!("next states:");
        for next in &state.next {
            let next_state = definition
                .states()
                .iter()
                .find(|candidate| candidate.name == next.0)
                .expect("definition transitions were validated when it was loaded");
            match next_state.summary.as_deref() {
                Some(summary) => println!("  - {}: {summary}", next.0),
                None => println!("  - {}", next.0),
            }
        }
    }
}

fn print_optional(label: &str, value: Option<&str>) {
    if let Some(value) = value {
        println!("{label}: {value}");
    }
}

fn print_steps(label: &str, steps: &[String]) {
    if steps.is_empty() {
        return;
    }

    println!("{label}:");
    for step in steps {
        println!("  - {step}");
    }
}

fn status_all() {
    let instances = Instance::load_all();
    if instances.is_empty() {
        println!("no instances found");
        return;
    }

    println!("instance | definition | state");
    for instance in instances {
        println!(
            "{} | {} | {}",
            instance.name(),
            instance.definition().name(),
            instance.state().0
        );
    }
}

fn move_instance(instance_name: InstanceName, movement: Move) {
    let instance = Instance::apply_saved_move(instance_name.as_str(), movement);
    println!(
        "instance {} is now in state {}",
        instance.name(),
        instance.state().0
    );
}
