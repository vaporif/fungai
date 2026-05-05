use bevy::prelude::*;
use rand::RngExt;

#[derive(Resource, Clone, Debug, Reflect)]
pub struct LaunchConfig {
    pub seed: u64,
}

pub fn default_seed() -> u64 {
    if cfg!(debug_assertions) {
        420
    } else {
        rand::rng().random::<u64>()
    }
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            seed: default_seed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(debug_assertions)]
    #[test]
    fn launch_config_default_seed_is_420_in_debug() {
        assert_eq!(LaunchConfig::default().seed, 420);
    }
}
