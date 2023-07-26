use std::path::PathBuf;

use walkdir::WalkDir;

use crate::{service::ServiceInformation, template::TemplateInfomation};

#[derive(Debug, Default, Clone)]
pub struct WorkDirectory {
    pub path: PathBuf,
    templates: Vec<TemplateInfomation>,
    services: Vec<ServiceInformation>,
}

#[allow(unused)]
impl WorkDirectory {
    pub fn new<T>(path: T) -> WorkDirectory
    where
        T: Into<PathBuf> + Clone,
    {
        let walk = |s: String| {
            {
                WalkDir::new({
                    let mut x = Into::<PathBuf>::into(path.clone());
                    x.push(s);
                    x
                })
                .into_iter()
                .map(|element| {
                    if let Ok(dir) = element {
                        if (dir.file_name().to_str().unwrap() != "manifest.toml") {
                            return None;
                        }
                        return Some(dir.into_path());
                    } else {
                        return None;
                    }
                })
            }
            .filter(Option::is_some)
            .map(Option::unwrap)
        };

        let templates = walk("templates".to_string())
            .map(TemplateInfomation::new)
            .filter(Result::is_ok)
            .map(Result::unwrap)
            .collect();

        let services = walk("services".to_string())
            .map(ServiceInformation::new)
            .filter(Result::is_ok)
            .map(Result::unwrap)
            .collect();

        WorkDirectory {
            path: path.into(),
            templates,
            services,
            ..Default::default()
        }
    }

    pub fn templates(self) -> Vec<TemplateInfomation> {
        self.templates
    }

    pub fn services(self) -> Vec<ServiceInformation> {
        self.services
    }

    pub fn data_directory(self) -> PathBuf {
        let mut path = self.path.clone();
        path.push("data");
        path
    }

    pub fn template_directory(self) -> PathBuf {
        let mut path = self.path.clone();
        path.push("templates");
        path
    }

    pub fn service_directory(self) -> PathBuf {
        let mut path = self.path.clone();
        path.push("services");
        path
    }
}
