use std::{
    fmt::Display, fs, ops::Deref, path::Path, str::FromStr, sync::Arc,
};
use async_trait::async_trait;
use error_stack::{Report, Result, ResultExt};
use lib::error::Error;
use serde::{Deserialize, Serialize};

use crate::services::{ServiceFactory, ServiceProvider};

#[derive(Debug, Serialize)]
pub struct ConfigServiceInner {
    pub tasks: TaskConfigs,
    pub environment: EnvironmentConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigService(Arc<ConfigServiceInner>);

impl<'de> Deserialize<'de> for ConfigService {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize, Default)]
        struct AdHocConfig {
            pub tasks: TaskConfigs,
            pub environment: EnvironmentConfig
        }

        let ad_hoc: AdHocConfig = serde::Deserialize::deserialize(deserializer)?;
        
        ConfigService::builder()
            .tasks(ad_hoc.tasks)
            .environment(ad_hoc.environment)
            .build()
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

impl FromStr for ConfigService {
    type Err = Report<Error>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let toml_config: ConfigService = toml::from_str(s)
            .change_context(Error::InvalidConfig)?;
        
        Ok(toml_config)
    }
}

impl Display for ConfigService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", toml::to_string(&self).unwrap())
    }
}

impl Deref for ConfigService {
    type Target = ConfigServiceInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ConfigService {
    pub fn inner(&self) -> Arc<ConfigServiceInner> {
        self.0.clone()
    }
}

#[buildstructor::buildstructor]
impl ConfigService {
    #[builder]
    pub fn new(
        tasks: Option<TaskConfigs>,
        environment: Option<EnvironmentConfig>,
    ) -> Result<Self, Error> {
        let inner = ConfigServiceInner {
            tasks: tasks.unwrap_or_default(),
            environment: environment.unwrap_or_default(),
        };

        Ok(ConfigService(Arc::new(inner)))
    }

    pub fn read_file(path: &Path) -> Result<Self, Error> {
        let config = fs::read_to_string(path)
            .change_context(Error::Unknown)?;

        config.parse()
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct TaskConfigs {
    pub fetcher: TaskConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct TaskConfig {
    pub interval: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct EnvironmentConfig {
    pub name: String,
    pub otlp_grpc_endpoint: String,
    pub otlp_http_endpoint: String,
}

#[async_trait]
impl ServiceFactory for ConfigService {
    async fn factory(_services: ServiceProvider) -> Result<Self, Error> {
        Err(Report::new(Error::Unknown)
            .attach_printable("Sir, You have no clue what you are doing. You must initialize config service manually."))
    }
}
