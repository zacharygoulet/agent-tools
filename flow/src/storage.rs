use std::{env, path::PathBuf};

use crate::{OptionContext, Result, ResultContext, user_error};

#[derive(Clone, Copy, Debug)]
pub enum Scope {
    Local,
    Global,
}

#[derive(Debug)]
pub struct Storage {
    local: PathBuf,
    global: PathBuf,
}

impl Storage {
    pub fn current() -> Result<Self> {
        let cwd = env::current_dir().context("getting the current directory")?;
        let state_home = if let Some(path) = env::var_os("XDG_STATE_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
        {
            path
        } else {
            let home = env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .context("neither XDG_STATE_HOME nor HOME identifies a state directory")?;
            PathBuf::from(home).join(".local/state")
        };
        Ok(Self { local: cwd.join(".flow"), global: state_home.join("flow") })
    }

    pub fn find_machine(&self, name: &str) -> Result<PathBuf> {
        self.find(name, "machine", "machines")
    }

    pub fn find_instance(&self, name: &str) -> Result<PathBuf> {
        self.find(name, "instance", "instances")
    }

    pub fn find_local_machine(&self, name: &str) -> Result<PathBuf> {
        let path = self.path(name, "machine", "machines", Scope::Local)?;
        if !path.exists() {
            return Err(user_error(format!(
                "local machine {name:?} not found at {}",
                path.display()
            )));
        }
        Ok(path)
    }

    pub fn find_global_machine(&self, name: &str) -> Result<PathBuf> {
        let path = self.global_machine_path(name)?;
        if !path.exists() {
            return Err(user_error(format!(
                "global machine {name:?} not found at {}",
                path.display()
            )));
        }
        Ok(path)
    }

    pub fn global_machine_path(&self, name: &str) -> Result<PathBuf> {
        self.path(name, "machine", "machines", Scope::Global)
    }

    pub fn new_instance_path(&self, name: &str, scope: Scope) -> Result<PathBuf> {
        let local = self.path(name, "instance", "instances", Scope::Local)?;
        let global = self.path(name, "instance", "instances", Scope::Global)?;
        for path in [&local, &global] {
            if path.exists() {
                return Err(user_error(format!(
                    "instance {name:?} already exists at {}",
                    path.display()
                )));
            }
        }
        Ok(match scope {
            Scope::Local => local,
            Scope::Global => global,
        })
    }

    fn find(&self, name: &str, kind: &str, directory: &str) -> Result<PathBuf> {
        let local = self.path(name, kind, directory, Scope::Local)?;
        if local.exists() {
            return Ok(local);
        }
        let global = self.path(name, kind, directory, Scope::Global)?;
        if global.exists() {
            return Ok(global);
        }
        Err(user_error(format!(
            "{kind} {name:?} not found (looked in {} and {})",
            local.display(),
            global.display()
        )))
    }

    fn path(&self, name: &str, kind: &str, directory: &str, scope: Scope) -> Result<PathBuf> {
        if name.is_empty() || name == "." || name.contains("..") || name.contains('/') || name.contains('\\')
        {
            return Err(user_error(format!("invalid {kind} name: {name:?}")));
        }
        let root = match scope {
            Scope::Local => &self.local,
            Scope::Global => &self.global,
        };
        Ok(root.join(directory).join(name))
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{Scope, Storage};

    fn storage_for_test(cwd: &Path, global_state_home: &Path) -> Storage {
        Storage { local: cwd.join(".flow"), global: global_state_home.join("flow") }
    }

    #[test]
    fn prefers_local_instance() {
        let root = tempfile::tempdir().unwrap();
        let local = root.path().join(".flow/instances/run");
        let global = root.path().join("global/flow/instances/run");
        fs::create_dir_all(local.parent().unwrap()).unwrap();
        fs::create_dir_all(global.parent().unwrap()).unwrap();
        fs::write(&local, "local").unwrap();
        fs::write(&global, "global").unwrap();

        let storage = storage_for_test(root.path(), &root.path().join("global"));
        assert_eq!(storage.find_instance("run").unwrap(), local);
    }

    #[test]
    fn falls_back_to_global_only_when_local_is_absent() {
        let root = tempfile::tempdir().unwrap();
        let global = root.path().join("global/flow/instances/run");
        fs::create_dir_all(global.parent().unwrap()).unwrap();
        fs::write(&global, "global").unwrap();

        let storage = storage_for_test(root.path(), &root.path().join("global"));
        assert_eq!(storage.find_instance("run").unwrap(), global);
    }

    #[test]
    fn reports_both_paths_when_missing() {
        let root = tempfile::tempdir().unwrap();
        let storage = storage_for_test(root.path(), &root.path().join("global"));
        let error = storage.find_instance("run").unwrap_err().to_string();
        assert!(error.contains(".flow/instances/run"));
        assert!(error.contains("global/flow/instances/run"));
    }

    #[test]
    fn rejects_names_that_escape_the_instances_directory() {
        let storage = storage_for_test(Path::new("."), Path::new("/tmp"));
        for name in ["", ".", "..", "../run", "nested/run", "nested\\run"] {
            assert!(storage.find_instance(name).is_err());
        }
    }

    #[test]
    fn finds_machine_locally_then_globally() {
        let root = tempfile::tempdir().unwrap();
        let local = root.path().join(".flow/machines/workflow");
        let global = root.path().join("global/flow/machines/workflow");
        fs::create_dir_all(local.parent().unwrap()).unwrap();
        fs::create_dir_all(global.parent().unwrap()).unwrap();
        fs::write(&global, "global").unwrap();

        let storage = storage_for_test(root.path(), &root.path().join("global"));
        assert_eq!(storage.find_machine("workflow").unwrap(), global);
        fs::write(&local, "local").unwrap();
        assert_eq!(storage.find_machine("workflow").unwrap(), local);
    }

    #[test]
    fn finds_machine_in_global_storage_only() {
        let root = tempfile::tempdir().unwrap();
        let local = root.path().join(".flow/machines/workflow");
        fs::create_dir_all(local.parent().unwrap()).unwrap();
        fs::write(&local, "local").unwrap();
        let storage = storage_for_test(root.path(), &root.path().join("global"));
        assert!(storage.find_global_machine("workflow").is_err());
        let global = storage.global_machine_path("workflow").unwrap();
        fs::create_dir_all(global.parent().unwrap()).unwrap();
        fs::write(&global, "global").unwrap();
        assert_eq!(storage.find_global_machine("workflow").unwrap(), global);
    }

    #[test]
    fn selects_local_or_global_creation_path() {
        let root = tempfile::tempdir().unwrap();
        let storage = storage_for_test(root.path(), &root.path().join("global"));
        assert_eq!(
            storage.new_instance_path("run", Scope::Local).unwrap(),
            root.path().join(".flow/instances/run")
        );
        assert_eq!(
            storage.new_instance_path("run", Scope::Global).unwrap(),
            root.path().join("global/flow/instances/run")
        );
    }

    #[test]
    fn rejects_creation_collisions_in_either_scope() {
        let root = tempfile::tempdir().unwrap();
        let storage = storage_for_test(root.path(), &root.path().join("global"));
        let local = root.path().join(".flow/instances/run");
        let global = root.path().join("global/flow/instances/run");
        fs::create_dir_all(local.parent().unwrap()).unwrap();
        fs::create_dir_all(global.parent().unwrap()).unwrap();

        fs::write(&local, "local").unwrap();
        assert!(storage.new_instance_path("run", Scope::Global).is_err());
        fs::remove_file(&local).unwrap();
        fs::write(&global, "global").unwrap();
        assert!(storage.new_instance_path("run", Scope::Local).is_err());
    }

    #[test]
    fn rejects_invalid_creation_names() {
        let root = tempfile::tempdir().unwrap();
        let storage = storage_for_test(root.path(), root.path());
        for name in ["", ".", "..", "../run", "nested/run", "nested\\run"] {
            assert!(storage.new_instance_path(name, Scope::Local).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn broken_local_symlink_allows_global_fallback() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let local = root.path().join(".flow/instances/run");
        let global = root.path().join("global/flow/instances/run");
        fs::create_dir_all(local.parent().unwrap()).unwrap();
        fs::create_dir_all(global.parent().unwrap()).unwrap();
        symlink("missing-target", &local).unwrap();
        fs::write(&global, "global").unwrap();

        let storage = storage_for_test(root.path(), &root.path().join("global"));
        assert_eq!(storage.find_instance("run").unwrap(), global);
    }
}
