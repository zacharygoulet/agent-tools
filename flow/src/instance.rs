use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    Definition, DefinitionName, InstanceName, StateName,
    file_writer::FileWriter,
    storage::{Scope, Storage},
};
use rust_utils::raise::{self, RaiseContext, RaiseExt};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Deserialize, Serialize, getset::Getters)]
#[getset(get = "pub")]
pub struct Instance {
    #[serde(skip, default = "InstanceName::placeholder")]
    name: InstanceName,
    #[serde(deserialize_with = "load_definition", serialize_with = "save_definition_name")]
    definition: Definition,
    state: StateName,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    context: BTreeMap<String, String>,
}

pub enum Move {
    Next(StateName),
    JumpTo(StateName),
}

pub enum ContextUpdate {
    Set { key: String, value: String },
    Remove { key: String },
}

#[derive(Debug)]
pub enum InstanceSavePolicy {
    Local,
    Global { copy_local_definition_to_global: bool },
}

impl InstanceSavePolicy {
    pub fn from_cli_args(global: bool, copy_definition: bool) -> Self {
        if global || copy_definition {
            Self::Global { copy_local_definition_to_global: copy_definition }
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

    fn prepare_definition(&self, definition: &Definition) {
        match self {
            Self::Local => {}
            Self::Global { copy_local_definition_to_global: true } => {
                definition.install_global_from_local_if_missing();
            }
            Self::Global { copy_local_definition_to_global: false } => definition.ensure_global_definition(),
        }
    }
}

impl Instance {
    pub fn new(name: InstanceName, definition: Definition) -> Self {
        definition.validate();
        let state = definition.initial_state().clone();
        let instance = Self { name, definition, state, context: BTreeMap::new() };
        instance.validate_current_state();
        instance
    }

    pub fn apply_move(&mut self, movement: Move) {
        let target = match movement {
            Move::Next(target) => {
                let current = self
                    .definition
                    .states()
                    .iter()
                    .find(|state| state.name == self.state.0)
                    .expect("current state was validated");
                if !current.next.iter().any(|next| next.0 == target.0) {
                    raise::raise(format!(
                        "state {:?} cannot move next to {:?} in definition {:?}",
                        self.state.0,
                        target.0,
                        self.definition.name()
                    ));
                }
                target
            }
            Move::JumpTo(target) => {
                if !self
                    .definition
                    .states()
                    .iter()
                    .any(|state| state.name == target.0)
                {
                    raise::raise(format!(
                        "cannot move to unknown state {:?} in definition {:?}",
                        target.0,
                        self.definition.name()
                    ));
                }
                target
            }
        };
        self.state = target;
    }

    pub fn move_to(mut self, movement: Move) -> Self {
        self.apply_move(movement);
        self.save_existing();
        self
    }

    pub fn update_context(mut self, update: ContextUpdate) -> Self {
        match update {
            ContextUpdate::Set { key, value } => {
                self.context.insert(key, value);
            }
            ContextUpdate::Remove { key } => {
                if self.context.remove(&key).is_none() {
                    raise::raise(format!(
                        "context key {key:?} not found in instance {:?}",
                        self.name
                    ));
                }
            }
        }
        self.save_existing();
        self
    }

    pub fn save_new(&self, policy: InstanceSavePolicy) -> PathBuf {
        let path = Storage::current().new_instance_path(self.name.as_str(), policy.scope());
        policy.prepare_definition(&self.definition);
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

    pub fn load_all() -> Vec<Self> {
        Storage::current()
            .instance_names()
            .iter()
            .map(|name| Self::load_from_name(name.as_str()))
            .collect()
    }

    fn save_existing(&self) {
        let path = Storage::current().find_instance(self.name.as_str());
        FileWriter::from(path).replace_existing(self.serialized_contents().as_bytes());
    }

    fn save_to_path(&self, path: &Path) {
        FileWriter::from(path.to_path_buf()).write_new(self.serialized_contents().as_bytes());
    }

    fn serialized_contents(&self) -> String {
        toml::to_string(self).raise_with_context(|| format!("serializing instance {:?}", self.name))
    }

    fn deserialize_from_path(path: &Path) -> Self {
        let contents = fs::read_to_string(path).raise_with_context(|| format!("reading {}", path.display()));
        toml::from_str(&contents).raise_with_context(|| format!("loading instance {}", path.display()))
    }

    fn validate_current_state(&self) {
        if !self
            .definition
            .states()
            .iter()
            .any(|state| state.name == self.state.0)
        {
            raise::raise(format!(
                "instance {:?} refers to unknown state {:?} in definition {:?}",
                self.name,
                self.state.0,
                self.definition.name()
            ));
        }
    }
}

fn load_definition<'de, D>(deserializer: D) -> std::result::Result<Definition, D::Error>
where
    D: Deserializer<'de>,
{
    let name = String::deserialize(deserializer)?;
    let name = DefinitionName::parse(name).map_err(serde::de::Error::custom)?;
    Ok(Definition::load_from_name(name.as_str()))
}

fn save_definition_name<S>(definition: &Definition, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(definition.name().as_str())
}

#[cfg(test)]
mod tests {
    use super::{Definition, Instance, Move, StateName};
    use crate::{DefinitionName, InstanceName, State};
    use indoc::indoc;
    use rust_utils::raise::catch_raised;

    fn raised_message<T: std::fmt::Debug>(operation: impl FnOnce() -> T) -> String {
        catch_raised(std::panic::AssertUnwindSafe(operation))
            .unwrap_err()
            .to_string()
    }

    fn state(name: &str, next: Vec<StateName>) -> State {
        State { name: name.to_owned(), next, summary: None, details: None, steps: Vec::new() }
    }

    fn instance_with_transitions() -> Instance {
        Instance::new(
            InstanceName::parse("run").unwrap(),
            Definition::new(
                DefinitionName::parse("workflow").unwrap(),
                StateName("Draft".into()),
                vec![
                    state("Draft", vec![StateName("Review".into())]),
                    state("Review", vec![]),
                    state("Done", vec![]),
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
    fn constructor_rejects_invalid_definition() {
        let definition: Definition = toml::from_str(indoc! {r#"
            initial_state = "Missing"
            [[states]]
            name = "Design"
            next = []
        "#})
        .unwrap();

        assert!(
            raised_message(|| Instance::new(InstanceName::parse("run").unwrap(), definition))
                .contains("unknown initial state")
        );
    }

    #[test]
    fn verifies_current_state_belongs_to_definition() {
        let mut instance = instance_with_transitions();
        instance.state = StateName("Missing".into());
        assert!(raised_message(|| instance.validate_current_state()).contains("Missing"));
    }
}
