use std::{
    fmt::{self, Write},
    path::Path,
};

use flow::{Autonomy, Definition, DefinitionName, GlobalGuidance, Instance};

pub struct InstanceStatus<'a>(pub &'a Instance);
pub struct InstancesTable<'a>(pub &'a [Instance]);
pub struct DefinitionsTable<'a>(pub &'a [Definition]);
pub struct StatesTable<'a>(pub &'a Definition);

impl fmt::Display for InstanceStatus<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let instance = self.0;
        let definition = instance.definition();
        let state = definition
            .states()
            .iter()
            .find(|state| state.name == instance.state().0)
            .expect("the instance state was validated when it was loaded");

        writeln!(formatter, "instance: {}", instance.name())?;
        if !instance.context().is_empty() {
            writeln!(formatter, "context:")?;
            formatter.write_str(
                &toml::to_string(instance.context()).expect("context strings serialize to TOML"),
            )?;
        }
        if *definition.use_global() {
            let guidance = GlobalGuidance::bundled();
            writeln!(formatter, "global:")?;
            write_optional(formatter, "details", guidance.details().as_deref())?;
            write_steps(formatter, guidance.steps())?;
        }
        writeln!(formatter, "definition: {}", definition.name())?;
        write_optional(formatter, "summary", definition.summary().as_deref())?;
        write_optional(formatter, "details", definition.details().as_deref())?;
        write_steps(formatter, definition.steps())?;
        writeln!(formatter, "state: {}", state.name)?;
        let autonomy = instance.current_autonomy();
        writeln!(formatter, "  autonomy: {autonomy}")?;
        writeln!(
            formatter,
            "    {}",
            match autonomy {
                Autonomy::Guided => "Pause for user approval at state boundaries and significant decisions.",
                Autonomy::Steered => "Proceed by default; ask on consequential choices and report progress.",
                Autonomy::Autonomous => "Continue independently; stop only for fundamental blockers.",
            }
        )?;
        write_optional(formatter, "summary", state.summary.as_deref())?;
        write_optional(formatter, "details", state.details.as_deref())?;
        write_steps(formatter, &state.steps)?;

        if !state.next.is_empty() {
            writeln!(formatter, "next states:")?;
            for next in &state.next {
                let next_state = definition
                    .states()
                    .iter()
                    .find(|candidate| candidate.name == next.0)
                    .expect("definition transitions were validated when it was loaded");
                match next_state.summary.as_deref() {
                    Some(summary) => writeln!(formatter, "  - {}: {summary}", next.0)?,
                    None => writeln!(formatter, "  - {}", next.0)?,
                }
            }
        }
        Ok(())
    }
}

fn write_optional(formatter: &mut fmt::Formatter<'_>, label: &str, value: Option<&str>) -> fmt::Result {
    if let Some(value) = value {
        let mut lines = value.trim_end_matches('\n').split('\n');
        writeln!(formatter, "  {label}: {}", lines.next().unwrap_or_default())?;
        for line in lines {
            writeln!(formatter, "    {line}")?;
        }
    }
    Ok(())
}

fn write_steps(formatter: &mut fmt::Formatter<'_>, steps: &[String]) -> fmt::Result {
    if steps.is_empty() {
        return Ok(());
    }
    writeln!(formatter, "  steps:")?;
    for step in steps {
        writeln!(formatter, "    - {step}")?;
    }
    Ok(())
}

impl fmt::Display for InstancesTable<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let instances = self.0;
        if instances.is_empty() {
            return writeln!(formatter, "no instances found");
        }

        let instance_width = instances
            .iter()
            .map(|instance| instance.name().as_str().len())
            .max()
            .unwrap_or(0)
            .max("INSTANCE".len());
        let definition_width = instances
            .iter()
            .map(|instance| instance.definition().name().as_str().len())
            .max()
            .unwrap_or(0)
            .max("DEFINITION".len());
        writeln!(
            formatter,
            "{:<instance_width$}  {:<definition_width$}  STATE",
            "INSTANCE", "DEFINITION"
        )?;
        writeln!(
            formatter,
            "{}  {}  -----",
            "-".repeat(instance_width),
            "-".repeat(definition_width)
        )?;
        for instance in instances {
            writeln!(
                formatter,
                "{:<instance_width$}  {:<definition_width$}  {}",
                instance.name(),
                instance.definition().name(),
                instance.state().0
            )?;
        }
        Ok(())
    }
}

impl fmt::Display for DefinitionsTable<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return writeln!(formatter, "no definitions found");
        }
        write_list_table(
            formatter,
            "DEFINITION",
            self.0.iter().map(|definition| {
                (
                    definition.name().as_str(),
                    definition.summary().as_deref().unwrap_or(""),
                )
            }),
        )
    }
}

impl fmt::Display for StatesTable<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_list_table(
            formatter,
            "STATE",
            self.0
                .states()
                .iter()
                .map(|state| (state.name.as_str(), state.summary.as_deref().unwrap_or(""))),
        )
    }
}

fn write_list_table<'a>(
    formatter: &mut fmt::Formatter<'_>,
    header: &str,
    rows: impl Iterator<Item = (&'a str, &'a str)>,
) -> fmt::Result {
    let rows: Vec<_> = rows.collect();
    let width = rows
        .iter()
        .map(|(name, _)| name.chars().count())
        .max()
        .unwrap_or(0)
        .max(header.len());
    writeln!(formatter, "{header:<width$}  SUMMARY")?;
    writeln!(formatter, "{}  -------", "-".repeat(width))?;
    for (name, summary) in rows {
        if summary.is_empty() {
            writeln!(formatter, "{name}")?;
        } else {
            writeln!(formatter, "{name:<width$}  {summary}")?;
        }
    }
    Ok(())
}

pub fn created_instance(instance: &Instance, path: &Path) -> String {
    format!("created instance {} at {}", instance.name(), path.display())
}

pub fn created_definition(name: &DefinitionName, path: &Path) -> String {
    format!("created definition {} at {}", name, path.display())
}

pub fn moved_instance(instance: &Instance) -> String {
    format!(
        "instance {} is now in state {}",
        instance.name(),
        instance.state().0
    )
}

pub fn autonomy_changed(level: Autonomy, changed: &[String]) -> String {
    let mut message = format!("set autonomy to {level} for states:\n");
    for name in changed {
        writeln!(message, "  - {name}").expect("writing to a String cannot fail");
    }
    message
}
