use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use indoc::indoc;

fn load_with_machine(machine_definition: Option<&str>) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::create_dir_all(root.join(".flow/instances")).unwrap();
    fs::write(
        root.join(".flow/instances/run"),
        indoc! {r#"
            machine = "workflow"
            state = "Design"
        "#},
    )
    .unwrap();

    if let Some(contents) = machine_definition {
        fs::create_dir_all(root.join(".flow/machines")).unwrap();
        fs::write(root.join(".flow/machines/workflow"), contents).unwrap();
    }

    run_cli(root, &["load", "run"])
}

fn run_cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_flow"))
        .args(args)
        .current_dir(root)
        .env("XDG_STATE_HOME", root.join("global"))
        .output()
        .unwrap()
}

fn write_local_machine(root: &Path) {
    write_machine(&root.join(".flow/machines/workflow"));
}

fn write_global_machine(root: &Path) {
    write_machine(&root.join("global/flow/machines/workflow"));
}

fn write_machine(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        indoc! {r#"
            initial_state = "Design"
            [[states]]
            name = "Design"
            next = []
        "#},
    )
    .unwrap();
}

fn write_movable_machine(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        indoc! {r#"
            initial_state = "Design"
            [[states]]
            name = "Design"
            next = ["Review"]
            [[states]]
            name = "Review"
            next = []
            [[states]]
            name = "Done"
            next = []
        "#},
    )
    .unwrap();
}

#[test]
fn loads_instance_with_separate_machine_file() {
    let output = load_with_machine(Some(indoc! {r#"
            initial_state = "Design"
            [[states]]
            name = "Design"
            next = []
        "#}));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        indoc! {r#"
            instance: run
            machine: workflow
            state: Design
        "#}
    );
}

#[test]
fn reports_missing_machine_from_instance_deserialization() {
    let output = load_with_machine(None);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains(r#"machine "workflow" not found"#), "{error}");
    assert!(error.contains(".flow/machines/workflow"), "{error}");
}

#[test]
fn reports_invalid_machine_from_instance_deserialization() {
    let output = load_with_machine(Some(indoc! {r#"
            initial_state = "Design"
            [[states]]
            name = "Design"
            next = ["Missing"]
        "#}));
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("unknown next state Missing"), "{error}");
    assert!(error.contains(".flow/machines/workflow"), "{error}");
}

#[test]
fn new_creates_local_instance_that_can_be_loaded() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_local_machine(root);

    let output = run_cli(root, &["new", "workflow", "run"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("created instance run at"));
    let instance_path = root.join(".flow/instances/run");
    assert_eq!(
        fs::read_to_string(&instance_path).unwrap(),
        indoc! {r#"
            machine = "workflow"
            state = "Design"
        "#}
    );

    let loaded = run_cli(root, &["load", "run"]);
    assert!(
        loaded.status.success(),
        "{}",
        String::from_utf8_lossy(&loaded.stderr)
    );
    assert!(String::from_utf8_lossy(&loaded.stdout).contains("state: Design"));
}

#[test]
fn next_and_jump_persist_local_instance_state() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_movable_machine(&root.join(".flow/machines/workflow"));
    assert!(run_cli(root, &["new", "workflow", "run"]).status.success());

    let next = run_cli(root, &["next", "run", "Review"]);
    assert!(next.status.success(), "{}", String::from_utf8_lossy(&next.stderr));
    let loaded = run_cli(root, &["load", "run"]);
    assert!(loaded.status.success());
    assert!(String::from_utf8_lossy(&loaded.stdout).contains("state: Review"));

    let jump = run_cli(root, &["jump", "run", "Done"]);
    assert!(jump.status.success(), "{}", String::from_utf8_lossy(&jump.stderr));
    assert_eq!(
        fs::read_to_string(root.join(".flow/instances/run")).unwrap(),
        indoc! {r#"
            machine = "workflow"
            state = "Done"
        "#}
    );
}

#[test]
fn rejected_moves_leave_instance_file_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_movable_machine(&root.join(".flow/machines/workflow"));
    assert!(run_cli(root, &["new", "workflow", "run"]).status.success());
    let path = root.join(".flow/instances/run");
    let original = fs::read_to_string(&path).unwrap();

    let next = run_cli(root, &["next", "run", "Done"]);
    assert!(!next.status.success());
    assert!(String::from_utf8_lossy(&next.stderr).contains("cannot move next"));
    let jump = run_cli(root, &["jump", "run", "Missing"]);
    assert!(!jump.status.success());
    assert!(String::from_utf8_lossy(&jump.stderr).contains("unknown state"));
    assert_eq!(fs::read_to_string(path).unwrap(), original);
}

#[test]
fn movement_updates_global_instance_without_creating_a_local_one() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_movable_machine(&root.join("global/flow/machines/workflow"));
    assert!(run_cli(root, &["new", "workflow", "run", "-g"]).status.success());

    let next = run_cli(root, &["next", "run", "Review"]);
    assert!(next.status.success(), "{}", String::from_utf8_lossy(&next.stderr));
    assert!(!root.join(".flow/instances/run").exists());
    assert!(
        fs::read_to_string(root.join("global/flow/instances/run"))
            .unwrap()
            .contains("state = \"Review\"")
    );
}

#[test]
fn new_rejects_collision_in_global_scope() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_local_machine(root);
    let existing = root.join("global/flow/instances/run");
    fs::create_dir_all(existing.parent().unwrap()).unwrap();
    fs::write(&existing, "original").unwrap();

    let output = run_cli(root, &["new", "workflow", "run"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
    assert!(!root.join(".flow/instances/run").exists());
    assert_eq!(fs::read_to_string(existing).unwrap(), "original");
}

#[test]
fn new_requires_machine_before_creating_instance_file() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();

    let output = run_cli(root, &["new", "workflow", "run"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("machine \"workflow\" not found"));
    assert!(!root.join(".flow/instances").exists());
}

#[test]
fn new_does_not_implement_generated_names_yet() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_local_machine(root);

    let unnamed = run_cli(root, &["new", "workflow"]);
    assert!(!unnamed.status.success());
    assert!(
        String::from_utf8_lossy(&unnamed.stderr).contains("automatic instance naming is not implemented")
    );
    assert!(!root.join(".flow/instances").exists());
}

#[test]
fn new_global_creates_instance_with_global_machine() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_global_machine(root);

    let output = run_cli(root, &["new", "workflow", "run", "-g"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(root.join("global/flow/instances/run")).unwrap(),
        indoc! {r#"
            machine = "workflow"
            state = "Design"
        "#}
    );
    assert!(!root.join(".flow/instances/run").exists());
    let loaded = run_cli(root, &["load", "run"]);
    assert!(
        loaded.status.success(),
        "{}",
        String::from_utf8_lossy(&loaded.stderr)
    );
    assert!(String::from_utf8_lossy(&loaded.stdout).contains("state: Design"));
}

#[test]
fn new_copy_machine_installs_local_definition_and_creates_global_instance() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_local_machine(root);
    let local_machine = root.join(".flow/machines/workflow");

    let output = run_cli(root, &["new", "workflow", "run", "-G"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(root.join("global/flow/machines/workflow")).unwrap(),
        fs::read_to_string(local_machine).unwrap()
    );
    assert!(!root.join(".flow/instances/run").exists());
    let loaded = run_cli(root, &["load", "run"]);
    assert!(
        loaded.status.success(),
        "{}",
        String::from_utf8_lossy(&loaded.stderr)
    );
}

#[test]
fn new_copy_machine_keeps_existing_global_definition() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_local_machine(root);
    let global_machine = root.join("global/flow/machines/workflow");
    fs::create_dir_all(global_machine.parent().unwrap()).unwrap();
    let existing = indoc! {r#"
        initial_state = "Review"
        [[states]]
        name = "Review"
        next = []
    "#};
    fs::write(&global_machine, existing).unwrap();

    let output = run_cli(root, &["new", "workflow", "run", "-G"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(global_machine).unwrap(), existing);
    assert!(root.join("global/flow/instances/run").is_file());
}

#[test]
fn new_copy_machine_checks_instance_collision_before_copy() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_local_machine(root);
    let existing = root.join(".flow/instances/run");
    fs::create_dir_all(existing.parent().unwrap()).unwrap();
    fs::write(&existing, "original").unwrap();

    let output = run_cli(root, &["new", "workflow", "run", "-G"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
    assert!(!root.join("global/flow/machines/workflow").exists());
    assert_eq!(fs::read_to_string(existing).unwrap(), "original");
}

#[test]
fn new_copy_machine_rejects_invalid_existing_global_definition() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_local_machine(root);
    let global_machine = root.join("global/flow/machines/workflow");
    fs::create_dir_all(global_machine.parent().unwrap()).unwrap();
    fs::write(&global_machine, "invalid toml = ").unwrap();

    let output = run_cli(root, &["new", "workflow", "run", "-G"]);
    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(global_machine).unwrap(), "invalid toml = ");
    assert!(!root.join("global/flow/instances/run").exists());
}

#[test]
fn new_global_requires_global_machine_even_if_local_exists() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_local_machine(root);

    let output = run_cli(root, &["new", "workflow", "run", "-g"]);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("global machine \"workflow\" not found"), "{error}");
    assert!(error.contains("global/flow/machines/workflow"), "{error}");
    assert!(!root.join("global/flow/instances/run").exists());
}

#[test]
fn new_global_rejects_invalid_global_machine() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    write_local_machine(root);
    let global_machine = root.join("global/flow/machines/workflow");
    fs::create_dir_all(global_machine.parent().unwrap()).unwrap();
    fs::write(&global_machine, "invalid toml = ").unwrap();

    let output = run_cli(root, &["new", "workflow", "run", "-g"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("global/flow/machines/workflow"));
    assert!(!root.join("global/flow/instances/run").exists());
}
