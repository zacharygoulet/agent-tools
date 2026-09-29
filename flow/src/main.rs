use clap::Parser;
use flow::{Instance, InstanceSavePolicy, Machine};
use rust_utils::raise::{self, RaiseExt};

#[derive(Parser)]
enum Cli {
    New {
        machine_name: String,
        instance_name: Option<String>,
        /// Save globally, requiring an existing global machine definition.
        #[arg(short, long)]
        global: bool,
        /// Save globally, copying the local machine if absent globally.
        #[arg(short = 'G', long = "copy-machine")]
        copy_machine: bool,
    },

    Load {
        instance_name: String,
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

fn new(machine_name: String, instance_name: Option<String>, global: bool, copy_machine: bool) {
    let Some(name) = instance_name else {
        raise::raise("automatic instance naming is not implemented yet");
    };

    let machine = Machine::load_from_name(&machine_name).raise();
    let instance = machine.into_new_instance(name);
    let path = instance
        .save_new(InstanceSavePolicy::from_cli_args(global, copy_machine))
        .raise();
    println!("created instance {} at {}", instance.name, path.display());
}

fn load(instance_name: String) {
    let instance = Instance::load_from_name(&instance_name).raise();

    println!(
        "instance: {}\nmachine: {}\nstate: {}",
        instance.name, instance.machine.name, instance.state.0
    );
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::Cli;

    #[test]
    fn parses_new_with_global_instance_storage() {
        let cli = Cli::try_parse_from(["flow", "new", "workflow", "run", "-g"]).unwrap();
        assert!(matches!(
            cli,
            Cli::New {
                machine_name,
                instance_name: Some(instance_name),
                global: true,
                copy_machine: false,
            } if machine_name == "workflow" && instance_name == "run"
        ));
    }

    #[test]
    fn parses_copy_machine_without_global_flag() {
        let cli = Cli::try_parse_from(["flow", "new", "workflow", "run", "-G"]).unwrap();
        assert!(matches!(cli, Cli::New { global: false, copy_machine: true, .. }));
    }

    #[test]
    fn parses_new_without_instance_name() {
        let cli = Cli::try_parse_from(["flow", "new", "workflow"]).unwrap();
        assert!(matches!(cli, Cli::New { instance_name: None, global: false, .. }));
    }

    #[test]
    fn load_does_not_accept_global_flag() {
        assert!(Cli::try_parse_from(["flow", "load", "run", "-g"]).is_err());
        assert!(matches!(
            Cli::try_parse_from(["flow", "load", "run"]).unwrap(),
            Cli::Load { instance_name } if instance_name == "run"
        ));
    }
}
