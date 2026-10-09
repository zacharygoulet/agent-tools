use clap::{Parser, Subcommand};
use flow::{
    Autonomy, ContextUpdate, Definition, DefinitionName, GlobalGuidance, Instance, InstanceName,
    InstanceSavePolicy, Move, Scope, StateName, Storage,
};
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

    List {
        #[command(subcommand)]
        target: ListTarget,
    },

    Context {
        #[command(subcommand)]
        action: ContextAction,
    },

    Autonomy {
        #[command(subcommand)]
        action: AutonomyAction,
    },

    NewDefinitionFromTemplate {
        definition_name: DefinitionName,
        #[arg(short, long)]
        global: bool,
    },
}

#[derive(Subcommand)]
enum ListTarget {
    Definitions,
    States { definition_name: DefinitionName },
}

#[derive(Subcommand)]
enum ContextAction {
    Set {
        instance_name: InstanceName,
        key: String,
        value: String,
    },
    Remove {
        instance_name: InstanceName,
        key: String,
    },
}

#[derive(Subcommand)]
enum AutonomyAction {
    Set {
        instance_name: InstanceName,
        level: Autonomy,
        state: String,
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

    println!("instance: {}", instance.name());
    if !instance.context().is_empty() {
        println!("context:");
        print!(
            "{}",
            toml::to_string(instance.context()).expect("context strings serialize to TOML")
        );
    }
    if *definition.use_global() {
        let guidance = GlobalGuidance::bundled();
        println!("global:");
        print_optional("details", guidance.details().as_deref());
        print_steps(guidance.steps());
    }
    println!("definition: {}", definition.name());
    print_optional("summary", definition.summary().as_deref());
    print_optional("details", definition.details().as_deref());
    print_steps(definition.steps());
    println!("state: {}", state.name);
    let autonomy = instance.current_autonomy();
    println!("  autonomy: {autonomy}");
    println!(
        "    {}",
        match autonomy {
            Autonomy::Guided => "Pause for user approval at state boundaries and significant decisions.",
            Autonomy::Steered => "Proceed by default; ask on consequential choices and report progress.",
            Autonomy::Autonomous => "Continue independently; stop only for fundamental blockers.",
        }
    );
    print_optional("summary", state.summary.as_deref());
    print_optional("details", state.details.as_deref());
    print_steps(&state.steps);

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
        let mut lines = value.trim_end_matches('\n').split('\n');
        println!("  {label}: {}", lines.next().unwrap_or_default());
        for line in lines {
            println!("    {line}");
        }
    }
}

fn print_steps(steps: &[String]) {
    if steps.is_empty() {
        return;
    }

    println!("  steps:");
    for step in steps {
        println!("    - {step}");
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

fn list(target: ListTarget) {
    match target {
        ListTarget::Definitions => {
            let names = Storage::current().definition_names();
            if names.is_empty() {
                println!("no definitions found");
                return;
            }
            println!("definition | summary");
            for name in names {
                let definition = Definition::load_from_name(name.as_str());
                println!("{} | {}", name, definition.summary().as_deref().unwrap_or(""));
            }
        }
        ListTarget::States { definition_name } => {
            let definition = Definition::load_from_name(definition_name.as_str());
            println!("state | summary");
            for state in definition.states() {
                println!("{} | {}", state.name, state.summary.as_deref().unwrap_or(""));
            }
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
            println!("set autonomy to {level} for states:");
            for name in changed {
                println!("  - {name}");
            }
        }
    }
}

fn move_instance(instance_name: InstanceName, movement: Move) {
    let instance = Instance::load_from_name(instance_name.as_str()).move_to(movement);
    println!(
        "instance {} is now in state {}",
        instance.name(),
        instance.state().0
    );
}
