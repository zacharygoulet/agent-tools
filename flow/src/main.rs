use clap::Parser;
use flow::{Instance, InstanceName, InstanceSavePolicy, Machine, MachineName};
use rust_utils::raise;

#[derive(Parser)]
enum Cli {
    New {
        machine_name: MachineName,
        instance_name: Option<InstanceName>,
        /// Save globally, requiring an existing global machine definition.
        #[arg(short, long)]
        global: bool,
        /// Save globally, copying the local machine if absent globally.
        #[arg(short = 'G', long = "copy-machine")]
        copy_machine: bool,
    },

    Load {
        instance_name: InstanceName,
    },
}

#[rust_utils::raise_handler]
fn main() {
    run(Cli::parse());
}

fn run(cli: Cli) {
    match cli {
        Cli::New { machine_name, instance_name, global, copy_machine } => {
            new(machine_name, instance_name, global, copy_machine)
        }
        Cli::Load { instance_name } => load(instance_name),
    }
}

fn new(machine_name: MachineName, instance_name: Option<InstanceName>, global: bool, copy_machine: bool) {
    let Some(name) = instance_name else {
        raise::raise("automatic instance naming is not implemented yet");
    };

    let machine = Machine::load_from_name(machine_name.as_str());
    let instance = machine.into_new_instance(name);
    let path = instance.save_new(InstanceSavePolicy::from_cli_args(global, copy_machine));
    println!("created instance {} at {}", instance.name, path.display());
}

fn load(instance_name: InstanceName) {
    let instance = Instance::load_from_name(instance_name.as_str());

    println!(
        "instance: {}\nmachine: {}\nstate: {}",
        instance.name, instance.machine.name, instance.state.0
    );
}
