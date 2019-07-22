use lazy_static::lazy_static;
use std::env;
use std::sync::Mutex;

lazy_static! {
    static ref RUNTIME_CONFIG: Mutex<Option<MutagenRuntimeConfig>> = Mutex::new(None);
}

#[derive(Copy, Clone)]
pub struct MutagenRuntimeConfig {
    pub mutation_id: u32,
}

impl MutagenRuntimeConfig {
    /// access the currently active runtime-config based on the environment variable `MUATION_ID`
    pub fn get_default() -> Self {
        let mut lock_guard = RUNTIME_CONFIG.lock().unwrap();
        match &*lock_guard {
            None => {
                // runtime config not initialized -> set default config based on env-var
                let env_config = MutagenRuntimeConfig::from_env();
                *lock_guard = Some(env_config);
                env_config
            }
            Some(config) => *config,
        }
    }

    fn from_env() -> Self {
        let mutation_id = env::var("MUTATION_ID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        MutagenRuntimeConfig { mutation_id }
    }

    pub fn is_mutation_active(&self, mutation_id: u32) -> bool {
        self.mutation_id == mutation_id
    }

    pub fn in_bounds(&self, mutator_id: u32, num_mutations: u32) -> bool {
        mutator_id < self.mutation_id && self.mutation_id < num_mutations + mutator_id
    }

    pub fn get_mutation<'a, T>(&self, mutator_id: u32, mutations: &'a [T]) -> Option<&'a T> {
        if self.mutation_id < mutator_id {
            return None;
        }
        let index = self.mutation_id - mutator_id;
        mutations.get(index as usize)
    }
}

/// module with functions used for isolated and exhaustive tests of the `#[mutate]` attribute
#[cfg(any(test, feature = "self_test"))]
mod test_tools {

    use super::*;
    use std::sync::Mutex;

    lazy_static! {
        static ref TEST_LOCK: Mutex<()> = Mutex::new(());
    }

    impl MutagenRuntimeConfig {
        /// sets the global `mutation_id` correctly before running the test and runs tests sequentially.
        ///
        /// The lock is required to ensure that set `mutation_id` is valid for the complete duration of the test case.
        pub fn test_with_mutation_id<F: FnOnce() -> ()>(mutation_id: u32, testcase: F) {
            let lock = TEST_LOCK.lock();
            MutagenRuntimeConfig::set_test_config(mutation_id);
            testcase();
            drop(lock); // drop here to extend lifetime of lock guard
        }

        pub fn with_mutation_id(mutation_id: u32) -> Self {
            MutagenRuntimeConfig { mutation_id }
        }

        pub fn set_test_config(mutation_id: u32) {
            *RUNTIME_CONFIG.lock().unwrap() =
                Some(MutagenRuntimeConfig::with_mutation_id(mutation_id));
        }

        pub fn clear_test_config() {
            *RUNTIME_CONFIG.lock().unwrap() = None;
        }
    }
}
