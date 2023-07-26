use std::fs::{self, File};

use clap::ArgMatches;
use prettytable::{row, Table};

use crate::{
    service::{ServiceInformation, ServiceStatus, StartResult},
    work::WorkDirectory,
};

pub struct CMD {
    pub quite: bool,
    pub json: bool,
    pub workingdir: String,
    pub matches: ArgMatches,
}

impl CMD {
    pub async fn list(self) {
        let services = self
            .matches
            .try_get_many::<String>("services")
            .unwrap_or_default();

        let workdir = WorkDirectory::new(&self.workingdir);

        let services = if services.is_none() {
            workdir.services().clone()
        } else {
            let arg_services = services
                .clone()
                .unwrap()
                .map(|ele| ele.to_string())
                .collect::<Vec<String>>();
            filt_services(arg_services, workdir.services().clone())
        };

        self.print(services);
    }
    pub async fn start(self) {
        let arg_services = self
            .matches
            .try_get_many::<String>("services")
            .unwrap_or_default()
            .unwrap_or_default()
            .map(|ele| ele.to_string())
            .collect::<Vec<String>>();

        let workingdir = WorkDirectory::new(&self.workingdir);

        let services = filt_services(arg_services, workingdir.services());

        let services = update_all_status(services)
            .await
            .into_iter()
            .filter(|ele| ele.status != ServiceStatus::Start)
            .collect();

        let services = start_all_services(services).await;

        let result = if self.json {
            let services: Vec<_> = services
                .into_iter()
                .map(|ele| (ele.0, ele.1.ok()))
                .collect();

            serde_json::to_string_pretty(&services).expect("Cannot serialized into json")
        } else {
            let mut table = Table::new();

            table.set_titles(row![
                "Service Name",
                "Start Result",
                "Current Service Status"
            ]);

            services.into_iter().for_each(|ele| {
                table.add_row(row![
                    ele.0.name,
                    ele.1
                        .map(|value| value.to_string())
                        .unwrap_or_else(|err| err.to_string()),
                    ele.0.status
                ]);
            });

            table.to_string()
        };

        print!("{}", result);
    }
    pub async fn stop(self) {}
    pub async fn status(self) {}
    pub async fn template(self) {
        let workingdir = WorkDirectory::new(self.workingdir);

        match self.matches.subcommand() {
            Some(("create", matches)) => {
                let name = matches
                    .try_get_many::<String>("name")
                    .unwrap_or_default()
                    .expect("Cannot get name")
                    .map(|ele| ele.to_string())
                    .collect::<Vec<String>>();

                let templates = name
                    .into_iter()
                    .map(|ele| {
                        let mut template = workingdir.clone().template_directory().clone();
                        template.push(ele);
                        template
                    })
                    .map(|path| fs::create_dir_all(&path).map(|_| path.clone()).ok())
                    .filter(|ele| ele.is_some())
                    .map(|ele| ele.unwrap())
                    .map(|path| {
                        let mut path = path.clone();
                        path.push("manifest.toml");

                        let file = File::create(&path).expect("Cannot create file");

                        // file.
                        // let is_ok=

                        (path.clone(), file.is_ok())
                    })
                    .collect::<Vec<_>>();

                let result = if self.json {
                    serde_json::to_string_pretty(&templates).expect("Cannot serialized into json")
                } else {
                    let mut table = Table::new();
                    table.set_titles(row!["Manifest Path", "Is Ok"]);
                    let templates = templates
                        .into_iter()
                        .map(|(path, is_ok)| row![path.to_str().unwrap(), is_ok]);

                    table.extend(templates);

                    table.to_string()
                };

                if !self.quite {
                    print!("{}", result);
                }
            }
            Some(("list", _)) => {
                let tem = workingdir
                    .templates()
                    .into_iter()
                    .map(|ele| {
                        let path = ele.clone();
                        (path.name, path.path.clone().to_owned())
                    })
                    .collect::<Vec<_>>();

                let result = if self.json {
                    serde_json::to_string_pretty(&tem).expect("Cannot serialized into json")
                } else {
                    let mut table = Table::new();
                    table.set_titles(row!["Template Name", "Template Path"]);

                    tem.into_iter().for_each(|(name, path)| {
                        table.add_row(row![name, path.to_str().unwrap()]);
                    });

                    table.to_string()
                };

                if !self.quite {
                    println!("{}", result)
                }
            }
            Some(("delete", matches)) => {}
            _ => {}
        }
    }
    pub async fn create(self) {}
    pub async fn delete(self) {}
    pub async fn init(self) {}

    fn print(self, v: Vec<ServiceInformation>) {
        if !self.quite {
            let result = if self.json {
                serde_json::to_string_pretty(&v).expect("Cannot serialized into json")
            } else {
                into_status_table(&v).to_string()
            };
            println!("{}", result);
        }
    }
}

async fn update_all_status(v: Vec<ServiceInformation>) -> Vec<ServiceInformation> {
    let mut vec = vec![];

    for ele in v {
        vec.push(ele.update_status().await)
    }

    vec
}

async fn start_all_services(
    v: Vec<ServiceInformation>,
) -> Vec<(ServiceInformation, Result<StartResult, anyhow::Error>)> {
    let before = v.clone();

    let mut after = vec![];

    for ele in v {
        after.push(ele.update_status().await)
    }

    let mut vec = vec![];

    for index in 0..before.len() {
        vec.push((
            after.get(index).unwrap().clone(),
            after.get(index).unwrap().clone().start().await,
        ))
    }

    vec
}

fn into_status_table(v: &Vec<ServiceInformation>) -> Table {
    let mut table = Table::new();
    table.set_titles(row!["Service Name", "Status"]);
    table.extend(v.iter().map(|ele| row![ele.name, ele.status.to_string()]));
    table
}

fn filt_services(
    arg_services: Vec<String>,
    mut services: Vec<ServiceInformation>,
) -> Vec<ServiceInformation> {
    services.retain(|ele| arg_services.contains(&ele.name));

    services
}
