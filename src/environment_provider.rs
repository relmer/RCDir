// environment_provider.rs — Environment variable abstraction
//
// Port of: EnvironmentProviderBase.h, EnvironmentProvider.h, EnvironmentProvider.cpp
// Provides a trait for env var access so Config can be tested with mock values.

/// Trait for environment variable access.
/// Enables unit testing Config without depending on actual env vars.
pub trait EnvironmentProvider {
    fn get_env_var(&self, name: &str) -> Option<String>;
}

/// Default implementation that reads from the actual process environment.
pub struct DefaultEnvironmentProvider;

impl EnvironmentProvider for DefaultEnvironmentProvider {
    fn get_env_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

/// Mock implementation for unit tests.
/// Stores preset key-value pairs.
#[cfg(test)]
pub struct MockEnvironmentProvider {
    vars: std::collections::HashMap<String, String>,
}

#[cfg(test)]
impl MockEnvironmentProvider {
    pub fn new() -> Self {
        MockEnvironmentProvider {
            vars: std::collections::HashMap::new(),
        }
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.vars.insert(name.into(), value.into());
    }
}

#[cfg(test)]
impl EnvironmentProvider for MockEnvironmentProvider {
    fn get_env_var(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_provider_reads_path() {
        let provider = DefaultEnvironmentProvider;
        // PATH should always exist on Windows
        assert!(provider.get_env_var("PATH").is_some());
    }

    #[test]
    fn default_provider_returns_none_for_missing() {
        let provider = DefaultEnvironmentProvider;
        assert!(provider.get_env_var("RCDIR_NONEXISTENT_VAR_12345").is_none());
    }

    #[test]
    fn mock_provider_returns_set_values() {
        let mut mock = MockEnvironmentProvider::new();
        mock.set("RCDIR", "W;D=Red");
        assert_eq!(mock.get_env_var("RCDIR"), Some("W;D=Red".into()));
    }

    #[test]
    fn mock_provider_returns_none_for_unset() {
        let mock = MockEnvironmentProvider::new();
        assert!(mock.get_env_var("RCDIR").is_none());
    }
}
