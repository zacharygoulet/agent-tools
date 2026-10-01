use std::{env, path::PathBuf};

use rust_utils::raise::{self, RaiseContext};

#[derive(Clone, Copy, Debug)]
enum StorageFile {
    Machine,
    Instance,
}

impl StorageFile {
    fn label(self) -> &'static str {
        match self {
            Self::Machine => "machine",
            Self::Instance => "instance",
        }
    }

    fn directory(self) -> &'static str {
        match self {
            Self::Machine => "machines",
            Self::Instance => "instances",
        }
    }
}

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
    pub fn current() -> Self {
        let cwd = env::current_dir().raise_with_context(|| "getting the current directory".into());
        let state_home = if let Some(path) = env::var_os("XDG_STATE_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
        {
            path
        } else {
            let home = env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .raise_with_context(|| "neither XDG_STATE_HOME nor HOME identifies a state directory".into());
            PathBuf::from(home).join(".local/state")
        };
        Self { local: cwd.join(".flow"), global: state_home.join("flow") }
    }

    pub fn find_machine(&self, name: &str) -> PathBuf {
        self.find(name, StorageFile::Machine)
    }

    pub fn find_instance(&self, name: &str) -> PathBuf {
        self.find(name, StorageFile::Instance)
    }

    pub fn find_local_machine(&self, name: &str) -> PathBuf {
        let path = self.path(name, StorageFile::Machine, Scope::Local);
        if !path.exists() {
            raise::raise(format!("local machine {name:?} not found at {}", path.display()));
        }
        path
    }

    pub fn find_global_machine(&self, name: &str) -> PathBuf {
        let path = self.global_machine_path(name);
        if !path.exists() {
            raise::raise(format!("global machine {name:?} not found at {}", path.display()));
        }
        path
    }

    pub fn global_machine_path(&self, name: &str) -> PathBuf {
        self.path(name, StorageFile::Machine, Scope::Global)
    }

    pub fn new_instance_path(&self, name: &str, scope: Scope) -> PathBuf {
        let local = self.path(name, StorageFile::Instance, Scope::Local);
        let global = self.path(name, StorageFile::Instance, Scope::Global);
        for path in [&local, &global] {
            if path.exists() {
                raise::raise(format!("instance {name:?} already exists at {}", path.display()));
            }
        }
        match scope {
            Scope::Local => local,
            Scope::Global => global,
        }
    }

    fn find(&self, name: &str, storage_file: StorageFile) -> PathBuf {
        let local = self.path(name, storage_file, Scope::Local);
        if local.exists() {
            return local;
        }
        let global = self.path(name, storage_file, Scope::Global);
        if global.exists() {
            return global;
        }
        raise::raise(format!(
            "{} {name:?} not found (looked in {} and {})",
            storage_file.label(),
            local.display(),
            global.display()
        ));
    }

    fn path(&self, name: &str, storage_file: StorageFile, scope: Scope) -> PathBuf {
        if name.is_empty() || name == "." || name.contains("..") || name.contains('/') || name.contains('\\')
        {
            raise::raise(format!("invalid {} name: {name:?}", storage_file.label()));
        }
        let root = match scope {
            Scope::Local => &self.local,
            Scope::Global => &self.global,
        };
        root.join(storage_file.directory()).join(name)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{Scope, Storage};
    use rust_utils::raise::catch_raised;

    fn raised_message<T: std::fmt::Debug>(operation: impl FnOnce() -> T) -> String {
        catch_raised(std::panic::AssertUnwindSafe(operation))
            .unwrap_err()
            .to_string()
    }

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
        assert_eq!(storage.find_instance("run"), local);
    }

    #[test]
    fn reports_both_paths_when_missing() {
        let root = tempfile::tempdir().unwrap();
        let storage = storage_for_test(root.path(), &root.path().join("global"));
        let error = raised_message(|| storage.find_instance("run"));
        assert!(error.contains(".flow/instances/run"));
        assert!(error.contains("global/flow/instances/run"));
    }

    #[test]
    fn rejects_names_that_escape_the_instances_directory() {
        let storage = storage_for_test(Path::new("."), Path::new("/tmp"));
        for name in ["", ".", "..", "../run", "nested/run", "nested\\run"] {
            assert!(catch_raised(|| storage.find_instance(name)).is_err());
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
        assert_eq!(storage.find_machine("workflow"), global);
        fs::write(&local, "local").unwrap();
        assert_eq!(storage.find_machine("workflow"), local);
    }

    #[test]
    fn finds_machine_in_global_storage_only() {
        let root = tempfile::tempdir().unwrap();
        let local = root.path().join(".flow/machines/workflow");
        fs::create_dir_all(local.parent().unwrap()).unwrap();
        fs::write(&local, "local").unwrap();
        let storage = storage_for_test(root.path(), &root.path().join("global"));
        assert!(catch_raised(|| storage.find_global_machine("workflow")).is_err());
        let global = storage.global_machine_path("workflow");
        fs::create_dir_all(global.parent().unwrap()).unwrap();
        fs::write(&global, "global").unwrap();
        assert_eq!(storage.find_global_machine("workflow"), global);
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
        assert!(catch_raised(|| storage.new_instance_path("run", Scope::Global)).is_err());
        fs::remove_file(&local).unwrap();
        fs::write(&global, "global").unwrap();
        assert!(catch_raised(|| storage.new_instance_path("run", Scope::Local)).is_err());
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
        assert_eq!(storage.find_instance("run"), global);
    }
}
