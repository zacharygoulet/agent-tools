use std::{fs, io::Write, path::PathBuf};

use rust_utils::raise::RaiseContext;
use tempfile::NamedTempFile;

pub struct FileWriter(PathBuf);

impl From<PathBuf> for FileWriter {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

impl FileWriter {
    pub fn write_new(&self, contents: &[u8]) {
        let directory = self
            .0
            .parent()
            .raise_with_context(|| "file path has no parent directory".into());
        fs::create_dir_all(directory).raise_with_context(|| format!("creating {}", directory.display()));

        let mut temporary = NamedTempFile::new_in(directory)
            .raise_with_context(|| format!("creating temporary file in {}", directory.display()));
        temporary
            .write_all(contents)
            .raise_with_context(|| format!("writing temporary file for {}", self.0.display()));
        temporary
            .as_file()
            .sync_all()
            .raise_with_context(|| format!("syncing temporary file for {}", self.0.display()));
        temporary
            .persist_noclobber(&self.0)
            .raise_with_context(|| format!("creating file {} without overwriting", self.0.display()));
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use indoc::indoc;
    use rust_utils::raise::catch_raised;

    use super::FileWriter;

    #[test]
    fn writes_new_file_without_overwriting() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("instances/run");
        let contents = indoc! {r#"
            machine = "workflow"
            state = "Design"
        "#};
        let writer = FileWriter::from(path.clone());

        writer.write_new(contents.as_bytes());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        assert!(catch_raised(|| writer.write_new(b"different")).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn does_not_overwrite_dangling_symlink() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("run");
        symlink("missing-target", &path).unwrap();

        assert!(catch_raised(|| FileWriter::from(path.clone()).write_new(b"instance")).is_err());
        assert!(fs::symlink_metadata(path).unwrap().file_type().is_symlink());
    }
}
