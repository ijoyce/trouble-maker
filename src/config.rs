use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FailureType {
    Error,
    Delay,
    Timeout,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Failure {
    pub path: String,
    pub failure_type: FailureType,
    pub frequency: f32,
    pub delay: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub failures: Vec<Failure>,
    pub listener_address: String,
    pub proxy_address: String,
}

impl Configuration {
    pub fn print(&self) {
        for f in &self.failures {
            info!("{:?}", f);
        }
    }
}

pub fn init() -> Configuration {
    let mut config = config::Config::default();
    config
        .merge(config::File::with_name("Configuration"))
        .unwrap();
    let config = config.try_into::<Configuration>().unwrap();

    config
}
