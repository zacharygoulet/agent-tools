use std::panic::{UnwindSafe, catch_unwind};

use anyhow::{Result, bail};
use clap::Parser;
use flow::{Instance, InstanceSavePolicy, Machine};

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

fn main() -> Result<()> {
    catch_panics(|| run(Cli::parse()))
}

fn catch_panics(action: impl FnOnce() -> Result<()> + UnwindSafe) -> Result<()> {
    match catch_unwind(action) {
        Ok(result) => result,
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("non-string panic payload");
            bail!("internal panic: {message}")
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli {
        Cli::New { machine_name, instance_name, global, copy_machine } => {
            new(machine_name, instance_name, global, copy_machine)
        }
        Cli::Load { instance_name } => load(instance_name),
    }
}

fn new(machine_name: String, instance_name: Option<String>, global: bool, copy_machine: bool) -> Result<()> {
    let Some(name) = instance_name else {
        bail!("automatic instance naming is not implemented yet");
    };

    let machine = Machine::load_from_name(&machine_name)?;
    let instance = machine.into_new_instance(name);
    let path = instance.save_new(InstanceSavePolicy::from_cli_args(global, copy_machine))?;
    println!("created instance {} at {}", instance.name, path.display());
    Ok(())
}

fn load(instance_name: String) -> Result<()> {
    let instance = Instance::load_from_name(&instance_name)?;

    println!(
        "instance: {}\nmachine: {}\nstate: {}",
        instance.name, instance.machine.name, instance.state.0
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, catch_panics};

    #[test]
    fn catches_panics_as_errors() {
        let error = catch_panics(|| -> anyhow::Result<()> { panic!("test panic") }).unwrap_err();
        assert_eq!(error.to_string(), "internal panic: test panic");
    }

    #[test]
    fn preserves_non_panic_errors() {
        let error = catch_panics(|| Err(anyhow::anyhow!("ordinary error"))).unwrap_err();
        assert_eq!(error.to_string(), "ordinary error");
    }

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
