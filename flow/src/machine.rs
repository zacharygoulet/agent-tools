use std::{collections::HashSet, fs, path::Path};

use crate::{Instance, file_writer::FileWriter, storage::Storage};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    #[serde(skip)]
    pub name: String,
    pub initial_state: StateName,
    pub states: Vec<State>,
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
    pub fn new(name: String, initial_state: StateName, states: Vec<State>) -> Result<Self> {
        let machine = Self { name, initial_state, states };
        machine.validate()?;
        Ok(machine)
    }

    pub fn into_new_instance(self, name: String) -> Instance {
        Instance::new(name, self)
    }

    pub fn validate(&self) -> Result<()> {
        let mut all_state_names = HashSet::new();
        for state in &self.states {
            if !all_state_names.insert(state.name.as_str()) {
                bail!("duplicate state: {}", state.name);
            }
        }

        if !all_state_names.contains(self.initial_state.0.as_str()) {
            bail!("unknown initial state: {}", self.initial_state.0);
        }

        for state in &self.states {
            for next in &state.next {
                if !all_state_names.contains(next.0.as_str()) {
                    bail!("state {} points to unknown next state {}", state.name, next.0);
                }
            }
        }
        Ok(())
    }

    pub fn load_from_name(name: &str) -> Result<Self> {
        Self::load_from_path(&Storage::current()?.find_machine(name)?)
    }

    pub fn load_global_from_name(name: &str) -> Result<Self> {
        Self::load_from_path(&Storage::current()?.find_global_machine(name)?)
    }

    pub fn ensure_global_definition(&self) -> Result<()> {
        Self::load_global_from_name(&self.name)?;
        Ok(())
    }

    pub fn install_global_from_local_if_missing(&self) -> Result<()> {
        let storage = Storage::current()?;
        let destination = storage.global_machine_path(&self.name)?;
        if destination.exists() {
            Self::load_from_path(&destination)?;
            return Ok(());
        }

        let source = storage.find_local_machine(&self.name)?;
        let contents =
            fs::read_to_string(&source).with_context(|| format!("reading {}", source.display()))?;
        Self::parse(&source, &contents)?;
        FileWriter::from(destination)
            .write_new(contents.as_bytes())
            .with_context(|| format!("installing global machine {:?}", self.name))?;
        Ok(())
    }

    fn load_from_path(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(path, &contents)
    }

    fn parse(path: &Path, contents: &str) -> Result<Self> {
        let definition: Machine =
            toml::from_str(contents).with_context(|| format!("parsing {}", path.display()))?;

        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("machine path {} has no UTF-8 filename", path.display()))?
            .to_owned();

        Self::new(name, definition.initial_state, definition.states)
            .with_context(|| format!("validating {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use indoc::indoc;

    use super::{Machine, State, StateName};

    #[test]
    fn validates_normal_next_states() {
        let machine = Machine {
            name: "example".into(),
            initial_state: StateName("draft".into()),
            states: vec![
                State { name: "draft".into(), next: vec![StateName("review".into())] },
                State { name: "review".into(), next: vec![] },
            ],
        };
        assert!(machine.validate().is_ok());
    }

    #[test]
    fn into_new_instance_uses_initial_state() {
        let machine = Machine::new(
            "workflow".into(),
            StateName("Design".into()),
            vec![
                State { name: "Implement".into(), next: vec![] },
                State { name: "Design".into(), next: vec![] },
            ],
        )
        .unwrap();

        let instance = machine.into_new_instance("run".into());
        assert_eq!(instance.name, "run");
        assert_eq!(instance.machine.name, "workflow");
        assert_eq!(instance.state.0, "Design");
    }

    #[test]
    fn rejects_duplicate_names() {
        let machine = Machine {
            name: "example".into(),
            initial_state: StateName("draft".into()),
            states: vec![
                State { name: "draft".into(), next: vec![] },
                State { name: "draft".into(), next: vec![] },
            ],
        };
        assert!(
            machine
                .validate()
                .unwrap_err()
                .to_string()
                .contains("duplicate state")
        );
    }

    #[test]
    fn rejects_unknown_next_state() {
        let machine = Machine {
            name: "example".into(),
            initial_state: StateName("draft".into()),
            states: vec![State { name: "draft".into(), next: vec![StateName("missing".into())] }],
        };
        assert!(
            machine
                .validate()
                .unwrap_err()
                .to_string()
                .contains("unknown next state")
        );
    }

    #[test]
    fn reads_valid_machine() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("workflow");
        fs::write(
            &path,
            indoc! {r#"
                initial_state = "Design"
                [[states]]
                name = "Design"
                next = ["Implement"]
                [[states]]
                name = "Implement"
                next = []
            "#},
        )
        .unwrap();

        let machine = Machine::load_from_path(&path).unwrap();
        assert_eq!(machine.name, "workflow");
        assert_eq!(machine.initial_state.0, "Design");
        assert_eq!(machine.states.len(), 2);
        assert_eq!(machine.states[0].next[0].0, "Implement");
    }

    #[test]
    fn rejects_unknown_initial_state() {
        let machine = Machine {
            name: "example".into(),
            initial_state: StateName("Missing".into()),
            states: vec![State { name: "Design".into(), next: vec![] }],
        };

        assert!(
            machine
                .validate()
                .unwrap_err()
                .to_string()
                .contains("unknown initial state")
        );
    }

    #[test]
    fn requires_initial_state_in_definition() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("workflow");
        fs::write(
            &path,
            indoc! {r#"
                [[states]]
                name = "Design"
                next = []
            "#},
        )
        .unwrap();

        assert!(format!("{:#}", Machine::load_from_path(&path).unwrap_err()).contains("initial_state"));
    }

    #[test]
    fn constructor_rejects_invalid_definition() {
        let error = Machine::new(
            "workflow".into(),
            StateName("Missing".into()),
            vec![State { name: "Design".into(), next: vec![] }],
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown initial state"));
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

        assert!(format!("{:#}", Machine::load_from_path(&path).unwrap_err()).contains("unknown field"));
    }

    #[test]
    fn rejects_invalid_next_state() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("workflow");
        fs::write(
            &path,
            indoc! {r#"
                initial_state = "Design"
                [[states]]
                name = "Design"
                next = ["Missing"]
            "#},
        )
        .unwrap();

        let error = Machine::load_from_path(&path).unwrap_err();
        assert!(format!("{error:#}").contains("unknown next state"));
    }
}
