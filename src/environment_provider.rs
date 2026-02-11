// environment_provider.rs — Environment variable abstraction
//
// Port of: EnvironmentProviderBase.h, EnvironmentProvider.h, EnvironmentProvider.cpp
// Provides a trait for env var access so Config can be tested with mock values.

/// Trait for environment variable access.
/// Enables unit testing Config without depending on actual env vars.
pub trait EnvironmentProvider {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  get_env_var
    //
    //  Returns the value of the named environment variable, if set.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn get_env_var(&self, name: &str) -> Option<String>;
}





/// Default implementation that reads from the actual process environment.
pub struct DefaultEnvironmentProvider;





////////////////////////////////////////////////////////////////////////////////
//
//  impl EnvironmentProvider for DefaultEnvironmentProvider
//
//  Reads the named environment variable from the process environment.
//
////////////////////////////////////////////////////////////////////////////////

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





////////////////////////////////////////////////////////////////////////////////
//
//  impl Default for MockEnvironmentProvider
//
//  Returns a new empty MockEnvironmentProvider.
//
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
impl Default for MockEnvironmentProvider {
    fn default() -> Self {
        Self::new()
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl MockEnvironmentProvider
//
//  Mock environment setup for unit tests.
//
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
impl MockEnvironmentProvider {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Creates a new empty MockEnvironmentProvider.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new() -> Self {
        MockEnvironmentProvider {
            vars: std::collections::HashMap::new(),
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  set
    //
    //  Sets a key-value pair in the mock environment.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn set(&mut self, name: &str, value: &str) {
        self.vars.insert(name.into(), value.into());
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl EnvironmentProvider for MockEnvironmentProvider
//
//  Returns the mock value for the named variable, if set.
//
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
impl EnvironmentProvider for MockEnvironmentProvider {
    fn get_env_var(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  default_provider_reads_path
    //
    //  Verifies DefaultEnvironmentProvider reads the PATH variable.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn default_provider_reads_path() {
        let provider = DefaultEnvironmentProvider;
        // PATH should always exist on Windows
        assert!(provider.get_env_var("PATH").is_some());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  default_provider_returns_none_for_missing
    //
    //  Verifies DefaultEnvironmentProvider returns None for missing vars.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn default_provider_returns_none_for_missing() {
        let provider = DefaultEnvironmentProvider;
        assert!(provider.get_env_var("RCDIR_NONEXISTENT_VAR_12345").is_none());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  mock_provider_returns_set_values
    //
    //  Verifies MockEnvironmentProvider returns preset values.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn mock_provider_returns_set_values() {
        let mut mock = MockEnvironmentProvider::new();
        mock.set("RCDIR", "W;D=Red");
        assert_eq!(mock.get_env_var("RCDIR"), Some("W;D=Red".into()));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  mock_provider_returns_none_for_unset
    //
    //  Verifies MockEnvironmentProvider returns None for unset vars.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn mock_provider_returns_none_for_unset() {
        let mock = MockEnvironmentProvider::new();
        assert!(mock.get_env_var("RCDIR").is_none());
    }
}
