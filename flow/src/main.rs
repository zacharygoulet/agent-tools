use clap::Parser;
use flow::{Instance, InstanceName, InstanceSavePolicy, Machine, MachineName, Move, Scope, StateName};
use rust_utils::raise::RaiseExt;
use uuid::Uuid;

const MACHINE_TEMPLATE: &str = include_str!("../templates/machine.toml");

#[derive(Parser)]
enum Cli {
    Start {
        machine_name: MachineName,
        instance_name: Option<InstanceName>,
        /// Save globally, requiring an existing global machine definition.
        #[arg(short, long)]
        global: bool,
        /// Save globally, copying the local machine if absent globally.
        #[arg(short = 'G', long = "copy-machine")]
        copy_machine: bool,
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

    NewMachineFromTemplate {
        machine_name: MachineName,
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
        Cli::Start { machine_name, instance_name, global, copy_machine } => {
            start(machine_name, instance_name, global, copy_machine)
        }
        Cli::NewMachineFromTemplate { machine_name, global } => {
            create_machine_from_template(machine_name, global)
        }
        Cli::Status { instance_name: Some(instance_name) } => status_instance(instance_name),
        Cli::Status { instance_name: None } => status_all(),
        Cli::Next { instance_name, target } => move_instance(instance_name, Move::Next(StateName(target))),
        Cli::Jump { instance_name, target } => move_instance(instance_name, Move::JumpTo(StateName(target))),
    }
}

fn start(machine_name: MachineName, instance_name: Option<InstanceName>, global: bool, copy_machine: bool) {
    let name = instance_name.unwrap_or_else(|| {
        InstanceName::parse(format!("{}-{}", machine_name, Uuid::new_v4().simple())).raise()
    });

    let machine = Machine::load_from_name(machine_name.as_str());
    let instance = machine.into_new_instance(name);
    let path = instance.save_new(InstanceSavePolicy::from_cli_args(global, copy_machine));
    println!("created instance {} at {}", instance.name(), path.display());
}

fn create_machine_from_template(machine_name: MachineName, global: bool) {
    let scope = if global { Scope::Global } else { Scope::Local };
    let path = Machine::create_from_template(machine_name.clone(), MACHINE_TEMPLATE, scope);
    println!("created machine {} at {}", machine_name, path.display());
}

fn status_instance(instance_name: InstanceName) {
    let instance = Instance::load_from_name(instance_name.as_str());
    let machine = instance.machine();
    let state = machine
        .states()
        .iter()
        .find(|state| state.name == instance.state().0)
        .expect("the instance state was validated when it was loaded");

    println!(
        "instance: {}\nmachine: {}\nstate: {}",
        instance.name(),
        machine.name(),
        instance.state().0
    );
    print_optional("machine summary", machine.summary().as_deref());
    print_optional("machine description", machine.description().as_deref());
    print_steps("machine required steps", machine.required_steps());
    print_steps("machine contextual steps", machine.contextual_steps());
    print_optional("state summary", state.summary.as_deref());
    print_optional("state description", state.description.as_deref());
    print_steps("state required steps", &state.required_steps);
    print_steps("state contextual steps", &state.contextual_steps);

    if !state.next.is_empty() {
        println!("next states:");
        for next in &state.next {
            let next_state = machine
                .states()
                .iter()
                .find(|candidate| candidate.name == next.0)
                .expect("machine transitions were validated when it was loaded");
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

    println!("instance | machine | state");
    for instance in instances {
        println!(
            "{} | {} | {}",
            instance.name(),
            instance.machine().name(),
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
