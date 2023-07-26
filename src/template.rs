use serde::Serialize;
use std::{
    fs::File,
    io::{BufReader, Read},
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
        let path: PathBuf = path.into();
        let file_reader = BufReader::new(File::open(path.clone())?);

        let mut file_content = String::new();
        file_reader.buffer().read_to_string(&mut file_content)?;

        let template = toml::from_str::<ManifestContent>(&file_content)
            .expect("Cannot deserialized into object");

        Ok(TemplateInfomation {
            name: template.name.clone(),
            path,
            template,
            ..Default::default()
        })
    }
}
