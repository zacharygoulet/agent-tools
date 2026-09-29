use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    Machine, StateName,
    file_writer::FileWriter,
    storage::{Scope, Storage},
};

use crate::{Result, ResultContext, user_error};
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

    fn prepare_machine(&self, machine: &Machine) -> Result<()> {
        match self {
            Self::Local => Ok(()),
            Self::Global { copy_local_machine_to_global: true } => {
                machine.install_global_from_local_if_missing()
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

    pub fn save_new(&self, policy: InstanceSavePolicy) -> Result<PathBuf> {
        let path = Storage::current()?.new_instance_path(&self.name, policy.scope())?;
        policy.prepare_machine(&self.machine)?;
        self.save_to_path(&path)?;
        Ok(path)
    }

    pub fn load_from_name(name: &str) -> Result<Self> {
        let mut instance = Self::load_from_path(&Storage::current()?.find_instance(name)?)?;
        instance.name = name.to_owned();
        instance.validate_current_state()?;
        Ok(instance)
    }

    fn save_to_path(&self, path: &Path) -> Result<()> {
        let contents =
            toml::to_string(self).with_context(|| format!("serializing instance {:?}", self.name))?;
        FileWriter::from(path.to_path_buf()).write_new(contents.as_bytes())
    }

    fn load_from_path(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&contents).with_context(|| format!("loading instance {}", path.display()))
    }

    fn validate_current_state(&self) -> Result<()> {
        if !self.machine.states.iter().any(|state| state.name == self.state.0) {
            return Err(user_error(format!(
                "instance {:?} refers to unknown state {:?} in machine {:?}",
                self.name, self.state.0, self.machine.name
            )));
        }
        Ok(())
    }
}

fn load_machine<'de, D>(deserializer: D) -> std::result::Result<Machine, D::Error>
where
    D: Deserializer<'de>,
{
    let name = String::deserialize(deserializer)?;
    Machine::load_from_name(&name).map_err(|error| serde::de::Error::custom(format!("{error:#}")))
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
        )
        .unwrap();

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

        assert!(instance.validate_current_state().is_ok());
        instance.state = StateName("Missing".into());
        assert!(
            instance
                .validate_current_state()
                .unwrap_err()
                .to_string()
                .contains("Missing")
        );
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

        let error = Instance::load_from_path(&path).unwrap_err();
        assert!(error.to_string().contains("loading instance"));
        assert!(format!("{error:#}").contains("machine"));
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

        let error = Instance::load_from_path(&path).unwrap_err();
        assert!(error.to_string().contains("loading instance"));
    }

    #[test]
    fn reports_unreadable_instance() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing");

        let error = Instance::load_from_path(&path).unwrap_err();
        assert!(error.to_string().contains("reading"));
    }
}
