use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
    process::Command,
};

use clap::ArgMatches;
use prettytable::{row, Table};

use crate::{
    manifest::ManifestContent,
    service::{ServiceInformation, ServiceStatus, StartResult, StopResult},
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
        let workdir = WorkDirectory::new(&self.workingdir);

        let services = workdir.services().clone();

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

        let services = filt_services(arg_services, workingdir.clone().services());

        let services = update_all_status(services)
            .await
            .into_iter()
            .filter(|ele| ele.status != ServiceStatus::Start)
            .collect();

        let services = start_all_services(services, workingdir.data_directory()).await;

        if self.quite {
            return;
        }

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
    pub async fn stop(self) {
        let services = self
            .matches
            .try_get_many::<String>("service")
            .unwrap_or_default()
            .expect("Cannot get service list")
            .map(|ele| ele.to_string())
            .collect::<Vec<String>>();

        let workdir = WorkDirectory::new(self.workingdir);

        let mut workdir_services = workdir.clone().services();

        workdir_services.retain(|ele| services.contains(&ele.name));

        let mut vec = vec![];
        for ele in workdir_services {
            vec.push((ele.clone(), ele.stop().await));
        }

        if self.quite {
            return;
        }

        let vec = vec
            .into_iter()
            .map(|ele| {
                (
                    ele.0.name,
                    if ele.1.is_err() {
                        ele.1.unwrap_err().to_string()
                    } else {
                        ele.1.unwrap().to_string()
                    },
                )
            })
            .collect::<Vec<_>>();

        let result = if self.json {
            serde_json::to_string_pretty(&vec).expect("Cannot serialized into json")
        } else {
            let mut table = Table::new();
            table.set_titles(row!["Service Name", "Stop Result"]);
            table.extend(
                vec.into_iter()
                    .map(|ele| row![ele.0, ele.1])
                    .collect::<Vec<_>>(),
            );
            table.to_string()
        };

        print!("{}", result)
    }

    pub async fn status(self) {
        let services = self
            .matches
            .try_get_many::<String>("service")
            .unwrap_or_default();

        let services = if services.is_none() {
            vec![]
        } else {
            services
                .unwrap()
                .map(|ele| ele.to_string())
                .collect::<Vec<String>>()
        };

        let workingdir = WorkDirectory::new(self.workingdir);

        let workingdir_service = if services.len() == 0 {
            workingdir.services().clone()
        } else {
            let mut v = workingdir.services().clone().clone();
            v.retain(|ele| services.contains(&ele.name));
            v
        };

        let mut vec = vec![];
        for ele in workingdir_service {
            vec.push((
                ele.clone().name,
                ele.update_status().await.status.to_string(),
            ))
        }

        let result = if self.json {
            serde_json::to_string_pretty(&vec).expect("Cannot serialized into json")
        } else {
            let mut table = Table::new();
            table.set_titles(row!["Serivice Name", "Service Status"]);
            let vec = vec
                .into_iter()
                .map(|(name, status)| row![name, status])
                .collect::<Vec<_>>();
            table.extend(vec);
            table.to_string()
        };

        print!("{}", result)
    }

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

                        let mut name = path.clone();
                        name.pop();

                        let default = ManifestContent {
                            ..Default::default()
                        };
                        let default =
                            toml::to_string_pretty(&default).expect("Cannot serialized into toml");

                        let buffer = BufWriter::new(file).write_all(default.as_bytes());

                        (path.clone(), buffer.is_ok())
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
            Some(("delete", matches)) => {
                let name = matches
                    .try_get_many::<String>("name")
                    .unwrap_or_default()
                    .expect("Cannot get name")
                    .map(|ele| ele.to_string())
                    .collect::<Vec<String>>();
                let mut tem = workingdir.templates().clone();

                tem.retain(|ele| name.contains(&ele.name));
                let action_res = tem
                    .into_iter()
                    .map(|ele| {
                        let mut path = ele.path.clone();
                        path.pop();
                        (ele.name, path)
                    })
                    .map(|ele| {
                        if fs::remove_dir_all(ele.1).is_err() {
                            (ele.0, false)
                        } else {
                            (ele.0, true)
                        }
                    })
                    .collect::<Vec<_>>();

                if self.quite {
                    return;
                }

                let r = if self.json {
                    serde_json::to_string_pretty(&action_res).expect("Cannot serialized into json")
                } else {
                    let mut table = Table::new();
                    table.set_titles(row!["Template Name", "Action Result"]);
                    let r = action_res
                        .into_iter()
                        .map(|ele| row![ele.0, ele.1])
                        .collect::<Vec<_>>();

                    table.extend(r);
                    table.to_string()
                };

                println!("{}", r)
            }
            _ => {}
        }
    }
    pub async fn create(self) {
        let service_name = self.matches.get_one::<String>("name").unwrap();
        let template_name = self.matches.get_one::<String>("template").unwrap();
        let workdir = WorkDirectory::new(self.workingdir);

        let templates = workdir
            .clone()
            .templates()
            .into_iter()
            .find(|ele| ele.name == *template_name);

        let service = workdir
            .clone()
            .services()
            .into_iter()
            .find(|ele| ele.name == *service_name);

        let mut action_result = (service_name, template_name, false, "".to_string());

        if templates.is_none() {
            println!("There is no template with the name {}", template_name);
            return;
        } else if service.is_some() {
            println!("There is already searvice with the name {}", service_name);
            return;
        }

        let template = templates.unwrap();

        let template_path = template.path.clone();

        // template_path.pop();

        let mut service_path = workdir.clone().service_directory().clone();
        service_path.push(service_name);

        fs::create_dir_all(&service_path).expect("Cannot create directory.");

        let strip = |template_path: PathBuf| {
            let mut root = workdir.clone().service_directory();
            root.pop();

            let p = PathBuf::from(template_path.strip_prefix(root).unwrap());
            let p = PathBuf::from(p.strip_prefix("templates").unwrap());

            let mut p = p
                .into_iter()
                .map(|ele| ele.to_str().unwrap().to_string())
                .collect::<Vec<_>>();

            if p.len() != 0 {
                p.remove(0);
            }
            let mut path = PathBuf::new();
            path.extend(p);

            path
        };

        for ele in walkdir::WalkDir::new(template_path) {
            let ele = ele.expect("Cannot visit");

            let striped = strip(ele.clone().into_path().clone());

            let mut target_path = service_path.clone();
            target_path.push(striped);

            if ele.file_type().is_dir() {
                fs::create_dir_all(target_path).expect("Cannot create directory");
            } else if ele.file_type().is_file() {
                fs_extra::file::copy(ele.path(), target_path, &fs_extra::file::CopyOptions::new())
                    .expect("Cannot copy file");
            } else {
                continue;
            }

            action_result.2 = true
        }

        if self.quite {
            return;
        }

        let result = if self.json {
            serde_json::to_string_pretty(&action_result).expect("Cannot serialized into json")
        } else {
            let mut table = Table::new();
            table.set_titles(row![
                "Service Name",
                "Template Name",
                "Result",
                "Infomation"
            ]);
            table.add_row(row![
                action_result.0,
                action_result.1,
                action_result.2,
                action_result.3
            ]);
            table.to_string()
        };

        println!("{}", result)
    }
    pub async fn delete(self) {
        let service_name = self.matches.get_one::<String>("name").unwrap().to_owned();
        let workdir = WorkDirectory::new(self.workingdir);
        let services = workdir.services().clone();
        let service = services.into_iter().find(|ele| ele.name == service_name);

        let mut action_result = (service_name, false, "".to_string());

        if service.is_none() {
            action_result.2 = "No such service found.".to_string()
        } else if let Some(service) = service {
            let service = service.update_status().await;

            let stopped = match service.status {
                ServiceStatus::Start => match service.stop().await {
                    Ok(result) => match result {
                        StopResult::StopFailed(ecode) => (false, ecode, "Stop failed".to_string()),
                        StopResult::PostStopFailed(ecode) => {
                            (false, ecode, "Post-stop failed".to_string())
                        }
                        _ => (true, 0, String::new()),
                    },
                    Err(_) => (false, -1, String::new()),
                },
                _ => (true, 0, String::new()),
            };

            action_result.1 = stopped.0;
            action_result.2 = if stopped.0 == true {
                format!("Stop succeed!")
            } else {
                format!("{},script exit code is {}", stopped.2, stopped.1)
            }
        }

        if self.quite {
            return;
        }

        let result = if self.json {
            serde_json::to_string_pretty(&action_result).expect("Cannot serialized into json")
        } else {
            let mut table = Table::new();
            table.set_titles(row!["Service Name", "Status", "Infomation"]);
            table.add_row(row![action_result.0, action_result.1, action_result.2]);
            table.to_string()
        };
        print!("{}", result)
    }

    pub async fn init(self) {
        let mut cmd = Command::new("bash");

        let cmd = cmd
            .arg("-c")
            .arg("git clone https://github.com/OakMemory/smcmanager-templates templates")
            .current_dir(self.workingdir);

        let output = cmd.output().expect("Cannot get output");

        if !output.status.success() {
            print!(
                "Initialize templates failed: {}",
                String::from_utf8(output.stderr).expect("Cannot into string")
            )
        }
    }

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

async fn start_all_services<T>(
    v: Vec<ServiceInformation>,
    data_dir: T,
) -> Vec<(ServiceInformation, Result<StartResult, anyhow::Error>)>
where
    T: Into<PathBuf> + Clone,
{
    let before = v.clone();

    let mut after = vec![];

    for ele in v {
        after.push(ele.update_status().await)
    }

    let mut vec = vec![];

    let data_dir: PathBuf = data_dir.into();

    for index in 0..before.len() {
        let mut data_dir = data_dir.clone();
        data_dir.push(after.get(index).unwrap().clone().name);

        vec.push((
            after.get(index).unwrap().clone(),
            after.get(index).unwrap().clone().start(data_dir).await,
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
