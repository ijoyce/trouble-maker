use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FailureType {
    Error,
    Delay,
    Timeout,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scenario {
    pub path: String,
    pub failure_type: FailureType,
    pub frequency: f32,
    pub delay: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub scenarios: Vec<Scenario>,
    pub listener_address: String,
    pub proxy_address: String,
    pub metrics_path: String,
    pub max_concurrent_requests: u32,
}

impl Configuration {
    pub fn print(&self) {
        for s in &self.scenarios {
            info!("{:?}", s);
        }
    }
}

pub fn init() -> Configuration {
    let mut config = config::Config::default();
    config
        .merge(config::File::with_name("Configuration"))
        .expect("Can't open Configuration.toml.");

    let config = config
        .try_into::<Configuration>()
        .expect("Can't deserialize Configuration.toml into a config struct.");

    if config.scenarios.len() == 0 {
        info!("No scenarios configured, so we'll proxy all traffic untouched.")
    }

    config
}
