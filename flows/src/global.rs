use rust_utils::raise::RaiseContext;
use serde::Deserialize;

#[derive(Deserialize, getset::Getters)]
#[serde(deny_unknown_fields)]
#[getset(get = "pub")]
pub struct GlobalGuidance {
    #[serde(default)]
    details: Option<String>,
    #[serde(default)]
    steps: Vec<String>,
}

impl GlobalGuidance {
    pub fn bundled() -> Self {
        toml::from_str(include_str!("../global.toml"))
            .raise_with_context(|| "parsing bundled global guidance".into())
    }
}

#[cfg(test)]
mod tests {
    use super::GlobalGuidance;

    #[test]
    fn bundled_guidance_loads() {
        let guidance = GlobalGuidance::bundled();
        assert!(!guidance.steps().is_empty());
    }

    #[test]
    fn rejects_summary_and_invalid_steps() {
        assert!(toml::from_str::<GlobalGuidance>("summary = 'unsupported'").is_err());
        assert!(toml::from_str::<GlobalGuidance>("steps = [1]").is_err());
    }
}
