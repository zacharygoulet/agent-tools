use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    InstanceName, Machine, MachineName, StateName,
    file_writer::FileWriter,
    storage::{Scope, Storage},
};
use rust_utils::raise::{self, RaiseContext, RaiseExt};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Deserialize, Serialize)]
pub struct Instance {
    // why do we need this? skip + default
    #[serde(skip, default = "InstanceName::placeholder")]
    pub name: InstanceName,
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
    pub fn new(name: InstanceName, machine: Machine) -> Self {
        let state = machine.initial_state.clone();
        Self { name, machine, state }
    }

    pub fn apply_move(&mut self, movement: Move) {
        self.validate_current_state();

        let target = match movement {
            Move::Next(target) => {
                let current = self
                    .machine
                    .states
                    .iter()
                    .find(|state| state.name == self.state.0)
                    .expect("current state was validated");
                if !current.next.iter().any(|next| next.0 == target.0) {
                    raise::raise(format!(
                        "state {:?} cannot move next to {:?} in machine {:?}",
                        self.state.0, target.0, self.machine.name
                    ));
                }
                target
            }
            Move::JumpTo(target) => target,
        };

        if !self.machine.states.iter().any(|state| state.name == target.0) {
            raise::raise(format!(
                "cannot move to unknown state {:?} in machine {:?}",
                target.0, self.machine.name
            ));
        }
        self.state = target;
    }

    pub fn save_new(&self, policy: InstanceSavePolicy) -> PathBuf {
        let path = Storage::current().new_instance_path(self.name.as_str(), policy.scope());
        policy.prepare_machine(&self.machine);
        self.save_to_path(&path);
        path
    }

    pub fn load_from_name(name: &str) -> Self {
        let name = InstanceName::parse(name).raise();
        let path = Storage::current().find_instance(name.as_str());
        let mut instance = Self::deserialize_from_path(&path);
        instance.name = name;
        instance.validate_current_state();
        instance
    }

    fn save_to_path(&self, path: &Path) {
        let contents =
            toml::to_string(self).raise_with_context(|| format!("serializing instance {:?}", self.name));
        FileWriter::from(path.to_path_buf()).write_new(contents.as_bytes());
    }

    fn deserialize_from_path(path: &Path) -> Self {
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
    let name = MachineName::parse(name).map_err(serde::de::Error::custom)?;
    Ok(Machine::load_from_name(name.as_str()))
}

fn save_machine_name<S>(machine: &Machine, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(machine.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::{Instance, Machine, Move, StateName};
    use crate::{InstanceName, MachineName, State};
    use rust_utils::raise::catch_raised;

    fn raised_message<T: std::fmt::Debug>(operation: impl FnOnce() -> T) -> String {
        catch_raised(std::panic::AssertUnwindSafe(operation))
            .unwrap_err()
            .to_string()
    }

    fn instance_with_transitions() -> Instance {
        Instance::new(
            InstanceName::parse("run").unwrap(),
            Machine::new(
                MachineName::parse("workflow").unwrap(),
                StateName("Draft".into()),
                vec![
                    State { name: "Draft".into(), next: vec![StateName("Review".into())] },
                    State { name: "Review".into(), next: vec![] },
                    State { name: "Done".into(), next: vec![] },
                ],
            ),
        )
    }

    #[test]
    fn next_follows_a_listed_transition() {
        let mut instance = instance_with_transitions();
        instance.apply_move(Move::Next(StateName("Review".into())));
        assert_eq!(instance.state.0, "Review");
    }

    #[test]
    fn next_rejects_an_unlisted_transition_without_changing_state() {
        let mut instance = instance_with_transitions();
        let error = raised_message(|| instance.apply_move(Move::Next(StateName("Done".into()))));
        assert!(error.contains("cannot move next"));
        assert_eq!(instance.state.0, "Draft");
    }

    #[test]
    fn jump_can_bypass_listed_transitions() {
        let mut instance = instance_with_transitions();
        instance.apply_move(Move::JumpTo(StateName("Done".into())));
        assert_eq!(instance.state.0, "Done");
    }

    #[test]
    fn jump_rejects_unknown_state_without_changing_state() {
        let mut instance = instance_with_transitions();
        let error = raised_message(|| instance.apply_move(Move::JumpTo(StateName("Missing".into()))));
        assert!(error.contains("unknown state"));
        assert_eq!(instance.state.0, "Draft");
    }

    #[test]
    fn verifies_current_state_belongs_to_machine() {
        let mut instance = Instance {
            name: super::InstanceName::parse("run").unwrap(),
            machine: Machine {
                name: MachineName::parse("workflow").unwrap(),
                initial_state: StateName("Design".into()),
                states: vec![State { name: "Design".into(), next: vec![] }],
            },
            state: StateName("Design".into()),
        };

        instance.state = StateName("Missing".into());
        assert!(raised_message(|| instance.validate_current_state()).contains("Missing"));
    }
}
