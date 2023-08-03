use serde::Serialize;
use std::{
    fs::{self},
    path::PathBuf,
};

use crate::manifest::ManifestContent;

#[derive(Debug, Default, Serialize, Clone)]
pub struct TemplateInfomation {
    pub name: String,
    pub path: PathBuf,
    pub template: ManifestContent,
}

impl TemplateInfomation {
    pub fn new<T>(path: T) -> anyhow::Result<TemplateInfomation>
    where
        T: Into<PathBuf>,
    {
        let mut path = path.into();
        let file_content = fs::read_to_string(&path)?;

        let template = toml::from_str::<ManifestContent>(&file_content)
            .expect("Cannot deserialized into object");

        path.pop();

        Ok(TemplateInfomation {
            name: path
                .into_iter()
                .last()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            path,
            template,
            ..Default::default()
        })
    }
}
