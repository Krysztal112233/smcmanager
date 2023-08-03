use core::fmt;
use std::{
    fmt::{Display, Formatter},
    fs::{self},
    path::PathBuf,
    process::ExitCode,
};

use crate::{executor::Executor, manifest::ManifestContent};
use anyhow::Ok;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServiceInformation {
    pub name: String,
    pub status: ServiceStatus,
    pub manifest: ManifestContent,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceStatus {
    Unknow,
    Start,
    Stop,
    Disable,
}

impl Display for ServiceStatus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for ServiceStatus {
    fn default() -> Self {
        ServiceStatus::Unknow
    }
}

impl ServiceInformation {
    pub fn new<T>(path: T) -> anyhow::Result<Self>
    where
        T: Into<PathBuf> + Clone,
    {
        let path: PathBuf = path.into();

        let file_content = fs::read_to_string(&path)?;

        let manifest = toml::from_str::<ManifestContent>(&file_content)?;

        Ok(ServiceInformation {
            name: manifest.name.clone(),
            ..Default::default()
        })
    }

    pub async fn update_status(self) -> Self {
        let mut status = ServiceStatus::Disable;

        if let Some(enable) = self.manifest.enable {
            if enable {
                let child = Executor::from(&self.manifest.scripts.health_check)
                    .exec()
                    .await
                    .expect(&format!(
                        "Cannot execute script {}",
                        &self.manifest.scripts.health_check
                    ));

                let (_, mut child) = Executor::output_reader(child).await;

                status = if child
                    .wait()
                    .expect("Cannot get exit status.")
                    .code()
                    .expect("Cannot get exit code.")
                    == 0
                {
                    ServiceStatus::Start
                } else {
                    ServiceStatus::Stop
                }
            }
        }
        return Self { status, ..self };
    }

    pub async fn start(self) -> anyhow::Result<StartResult> {
        if let Some(script) = self.manifest.scripts.pre_start {
            let mut child = Executor::from(script).exec().await?;
            let ecode = child.wait()?;
            if !&child.wait()?.success() {
                return Ok(StartResult::PreStartFailed(
                    ecode.code().expect("Canno get exit code"),
                ));
            };
        }

        let start = Executor::from(self.manifest.scripts.start);

        let ecode = start.exec().await?.wait()?;

        if !ecode.success() {
            Ok(StartResult::StartFailed(
                ecode.code().expect("Cannot get exit code"),
            ))
        } else {
            Ok(StartResult::Success)
        }
    }

    pub async fn stop(self) -> anyhow::Result<StopResult> {
        if let Some(stop) = self.manifest.scripts.stop {
            let stop = Executor::from(stop).exec().await?.wait()?;
            if !stop.success() {
                return Ok(StopResult::StopFailed(
                    stop.code().expect("Cannot get exit code"),
                ));
            }
        }

        if let Some(post_stop) = self.manifest.scripts.post_stop {
            let post_stop = Executor::from(post_stop).exec().await?.wait()?;
            if !post_stop.success() {
                return Ok(StopResult::PostStopFailed(
                    post_stop.code().expect("Cannot get exit code"),
                ));
            }
        }

        Ok(StopResult::Success)
    }
    pub async fn health_check(self) -> anyhow::Result<ExitCode> {
        todo!()
    }
}

#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum StopResult {
    Success,
    StopFailed(i32),
    PostStopFailed(i32),
}
impl Display for StopResult {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.clone())
    }
}

#[derive(Debug, Serialize)]
pub enum StartResult {
    Success,
    PreStartFailed(i32),
    StartFailed(i32),
}

impl Display for StartResult {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.clone())
    }
}

#[derive(Debug, Serialize)]
pub enum HealthCheckResult {
    Success(bool),
    RunFailed(i32),
}
impl Display for HealthCheckResult {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.clone())
    }
}
