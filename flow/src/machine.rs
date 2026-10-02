use std::{collections::HashSet, fs, path::Path};

use crate::{Instance, InstanceName, MachineName, file_writer::FileWriter, storage::Storage};
use rust_utils::raise::{self, RaiseContext, RaiseExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, getset::Getters)]
#[serde(deny_unknown_fields)]
#[getset(get = "pub")]
pub struct Machine {
    #[serde(skip, default = "MachineName::placeholder")]
    name: MachineName,
    initial_state: StateName,
    states: Vec<State>,
    // Possibly: guidance for the whole machine or for entering/leaving states.
    // Possibly: names of expected context entries; context values belong to a run.
}

#[derive(Debug, Deserialize)]
pub struct State {
    pub name: String,
    pub next: Vec<StateName>,
    // Possibly: informational guidance for this state.
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StateName(pub String);

impl Machine {
    pub fn new(name: MachineName, initial_state: StateName, states: Vec<State>) -> Self {
        let machine = Self { name, initial_state, states };
        machine.validate_at(None);
        machine
    }

    pub fn into_new_instance(self, name: InstanceName) -> Instance {
        Instance::new(name, self)
    }

    pub fn validate(&self) {
        self.validate_at(None);
    }

    fn validate_at(&self, context: Option<&Path>) {
        let raise_validation_error = |message: String| match context {
            Some(path) => raise::raise(format!("validating {}: {message}", path.display())),
            None => raise::raise(message),
        };
        let mut all_state_names = HashSet::new();
        for state in &self.states {
            if state.name.trim().is_empty() {
                raise_validation_error("state names cannot be empty".to_owned());
            }
            if !all_state_names.insert(state.name.as_str()) {
                raise_validation_error(format!("duplicate state: {}", state.name));
            }
        }

        if !all_state_names.contains(self.initial_state.0.as_str()) {
            raise_validation_error(format!("unknown initial state: {}", self.initial_state.0));
        }

        for state in &self.states {
            for next in &state.next {
                if !all_state_names.contains(next.0.as_str()) {
                    raise_validation_error(format!(
                        "state {} points to unknown next state {}",
                        state.name, next.0
                    ));
                }
            }
        }
    }

    pub fn load_from_name(name: &str) -> Self {
        let name = MachineName::parse(name).raise();
        let path = Storage::current().find_machine(name.as_str());
        Self::parse_from_path(&path)
    }

    pub fn load_global_from_name(name: &str) -> Self {
        let name = MachineName::parse(name).raise();
        Self::parse_from_path(&Storage::current().find_global_machine(name.as_str()))
    }

    pub fn ensure_global_definition(&self) {
        Self::load_global_from_name(self.name.as_str());
    }

    pub fn install_global_from_local_if_missing(&self) {
        let storage = Storage::current();
        let destination = storage.global_machine_path(self.name.as_str());
        if destination.exists() {
            Self::parse_from_path(&destination);
            return;
        }

        let source = storage.find_local_machine(self.name.as_str());
        let contents =
            fs::read_to_string(&source).raise_with_context(|| format!("reading {}", source.display()));
        Self::parse(&source, &contents);
        FileWriter::from(destination).write_new(contents.as_bytes());
    }

    fn parse_from_path(path: &Path) -> Self {
        let contents = fs::read_to_string(path).raise_with_context(|| format!("reading {}", path.display()));
        Self::parse(path, &contents)
    }

    fn parse(path: &Path, contents: &str) -> Self {
        let definition: Machine =
            toml::from_str(contents).raise_with_context(|| format!("parsing {}", path.display()));

        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("machine definition paths must have a UTF-8 filename");
        let name = MachineName::parse(name).raise();

        let machine = Self { name, initial_state: definition.initial_state, states: definition.states };
        machine.validate_at(Some(path));
        machine
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use indoc::indoc;

    use super::{Machine, State, StateName};
    use crate::MachineName;
    use rust_utils::raise::catch_raised;

    fn raised_message<T: std::fmt::Debug>(operation: impl FnOnce() -> T) -> String {
        catch_raised(std::panic::AssertUnwindSafe(operation))
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn rejects_empty_state_names() {
        let machine = Machine {
            name: MachineName::parse("example").unwrap(),
            initial_state: StateName(" ".into()),
            states: vec![State { name: " ".into(), next: vec![] }],
        };
        assert!(raised_message(|| machine.validate()).contains("state names cannot be empty"));
    }

    #[test]
    fn rejects_duplicate_names() {
        let machine = Machine {
            name: MachineName::parse("example").unwrap(),
            initial_state: StateName("draft".into()),
            states: vec![
                State { name: "draft".into(), next: vec![] },
                State { name: "draft".into(), next: vec![] },
            ],
        };
        assert!(raised_message(|| machine.validate()).contains("duplicate state"));
    }

    #[test]
    fn rejects_unknown_next_state() {
        let machine = Machine {
            name: MachineName::parse("example").unwrap(),
            initial_state: StateName("draft".into()),
            states: vec![State { name: "draft".into(), next: vec![StateName("missing".into())] }],
        };
        assert!(raised_message(|| machine.validate()).contains("unknown next state"));
    }

    #[test]
    fn rejects_unknown_initial_state() {
        let machine = Machine {
            name: MachineName::parse("example").unwrap(),
            initial_state: StateName("Missing".into()),
            states: vec![State { name: "Design".into(), next: vec![] }],
        };

        assert!(raised_message(|| machine.validate()).contains("unknown initial state"));
    }

    #[test]
    fn rejects_redundant_name_in_definition() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("workflow");
        fs::write(
            &path,
            indoc! {r#"
                initial_state = "Design"
                name = "other"
                [[states]]
                name = "Design"
                next = []
            "#},
        )
        .unwrap();

        assert!(raised_message(|| Machine::parse_from_path(&path)).contains("unknown field"));
    }
}
