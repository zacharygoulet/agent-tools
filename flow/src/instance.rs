use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    Machine, StateName,
    file_writer::FileWriter,
    storage::{Scope, Storage},
};
use rust_utils::raise::{self, RaiseContext};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Deserialize, Serialize)]
pub struct Instance {
    #[serde(skip)]
    pub name: String,
    #[serde(deserialize_with = "load_machine", serialize_with = "save_machine_name")]
    pub machine: Machine,
    pub state: StateName,
    // Persistent context belongs to this run; its representation is undecided.
}

pub enum Move {
    Next(StateName),
    JumpTo(StateName),
}

#[derive(Debug)]
pub enum InstanceSavePolicy {
    Local,
    Global { copy_local_machine_to_global: bool },
}

impl InstanceSavePolicy {
    pub fn from_cli_args(global: bool, copy_machine: bool) -> Self {
        if global || copy_machine {
            Self::Global { copy_local_machine_to_global: copy_machine }
        } else {
            Self::Local
        }
    }

    fn scope(&self) -> Scope {
        match self {
            Self::Local => Scope::Local,
            Self::Global { .. } => Scope::Global,
        }
    }

    fn prepare_machine(&self, machine: &Machine) {
        match self {
            Self::Local => {}
            Self::Global { copy_local_machine_to_global: true } => {
                machine.install_global_from_local_if_missing();
            }
            Self::Global { copy_local_machine_to_global: false } => machine.ensure_global_definition(),
        }
    }
}

impl Instance {
    pub fn new(name: String, machine: Machine) -> Self {
        let state = machine.initial_state.clone();
        Self { name, machine, state }
    }

    pub fn save_new(&self, policy: InstanceSavePolicy) -> PathBuf {
        let path = Storage::current().new_instance_path(&self.name, policy.scope());
        policy.prepare_machine(&self.machine);
        self.save_to_path(&path);
        path
    }

    pub fn load_from_name(name: &str) -> Self {
        let mut instance = Self::load_from_path(&Storage::current().find_instance(name));
        instance.name = name.to_owned();
        instance.validate_current_state();
        instance
    }

    fn save_to_path(&self, path: &Path) {
        let contents =
            toml::to_string(self).raise_with_context(|| format!("serializing instance {:?}", self.name));
        FileWriter::from(path.to_path_buf()).write_new(contents.as_bytes());
    }

    fn load_from_path(path: &Path) -> Self {
        let contents = fs::read_to_string(path).raise_with_context(|| format!("reading {}", path.display()));
        toml::from_str(&contents).raise_with_context(|| format!("loading instance {}", path.display()))
    }

    fn validate_current_state(&self) {
        if !self.machine.states.iter().any(|state| state.name == self.state.0) {
            raise::raise(format!(
                "instance {:?} refers to unknown state {:?} in machine {:?}",
                self.name, self.state.0, self.machine.name
            ));
        }
    }
}

fn load_machine<'de, D>(deserializer: D) -> std::result::Result<Machine, D::Error>
where
    D: Deserializer<'de>,
{
    let name = String::deserialize(deserializer)?;
    Ok(Machine::load_from_name(&name))
}

fn save_machine_name<S>(machine: &Machine, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&machine.name)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use indoc::indoc;

    use super::{Instance, InstanceSavePolicy, Machine, StateName};
    use crate::State;
    use rust_utils::raise::catch_raised;

    fn raised_message<T: std::fmt::Debug>(operation: impl FnOnce() -> T) -> String {
        catch_raised(std::panic::AssertUnwindSafe(operation))
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn save_policy_from_cli_flags() {
        assert!(matches!(
            InstanceSavePolicy::from_cli_args(false, false),
            InstanceSavePolicy::Local
        ));
        assert!(matches!(
            InstanceSavePolicy::from_cli_args(true, false),
            InstanceSavePolicy::Global { copy_local_machine_to_global: false }
        ));
        assert!(matches!(
            InstanceSavePolicy::from_cli_args(false, true),
            InstanceSavePolicy::Global { copy_local_machine_to_global: true }
        ));
        assert!(matches!(
            InstanceSavePolicy::from_cli_args(true, true),
            InstanceSavePolicy::Global { copy_local_machine_to_global: true }
        ));
    }

    #[test]
    fn new_instance_starts_in_machines_initial_state() {
        let machine = Machine::new(
            "workflow".into(),
            StateName("Design".into()),
            vec![
                State { name: "Implement".into(), next: vec![] },
                State { name: "Design".into(), next: vec![] },
            ],
        );

        let instance = Instance::new("run".into(), machine);
        assert_eq!(instance.name, "run");
        assert_eq!(instance.machine.name, "workflow");
        assert_eq!(instance.state.0, "Design");
    }

    #[test]
    fn serializes_machine_by_name() {
        let instance = Instance {
            name: "run".into(),
            machine: Machine {
                name: "workflow".into(),
                initial_state: StateName("Design".into()),
                states: vec![State { name: "Design".into(), next: vec![] }],
            },
            state: StateName("Design".into()),
        };
        assert_eq!(
            toml::to_string(&instance).unwrap(),
            indoc! {r#"
                machine = "workflow"
                state = "Design"
            "#}
        );
    }

    #[test]
    fn verifies_current_state_belongs_to_machine() {
        let mut instance = Instance {
            name: "run".into(),
            machine: Machine {
                name: "workflow".into(),
                initial_state: StateName("Design".into()),
                states: vec![State { name: "Design".into(), next: vec![] }],
            },
            state: StateName("Design".into()),
        };

        instance.validate_current_state();
        instance.state = StateName("Missing".into());
        assert!(raised_message(|| instance.validate_current_state()).contains("Missing"));
    }

    #[test]
    fn reports_missing_machine_name() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("run");
        fs::write(
            &path,
            indoc! {r#"
                state = "Design"
            "#},
        )
        .unwrap();

        let error = raised_message(|| Instance::load_from_path(&path));
        assert!(error.contains("loading instance"));
        assert!(error.contains("machine"));
    }

    #[test]
    fn reports_malformed_toml() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("run");
        fs::write(
            &path,
            indoc! {r#"
                machine = [
            "#},
        )
        .unwrap();

        let error = raised_message(|| Instance::load_from_path(&path));
        assert!(error.contains("loading instance"));
    }

    #[test]
    fn reports_unreadable_instance() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing");

        let error = raised_message(|| Instance::load_from_path(&path));
        assert!(error.contains("reading"));
    }
}
