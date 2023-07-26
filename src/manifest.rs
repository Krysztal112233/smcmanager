use anyhow::{anyhow, Ok};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ManifestContent {
    pub name: String,
    pub enable: Option<bool>,
    pub scripts: ManifestContentScripts,
    #[serde(skip)]
    vars: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ManifestContentScripts {
    pub health_check: String,
    pub pre_start: Option<String>,
    pub start: String,
    pub stop: Option<String>,
    pub post_stop: Option<String>,
}

#[allow(unused)]
impl ManifestContent {
    pub fn new<T>(content: T) -> anyhow::Result<ManifestContent>
    where
        T: Into<String> + Clone,
    {
        let content: String = content.into();
        let mut manifest = toml::from_str::<ManifestContent>(content.as_str())?;

        manifest.vars = ManifestContent::extract_vars(content)?;

        Ok(manifest)
    }

    fn extract_vars<T>(content: T) -> anyhow::Result<Vec<String>>
    where
        T: Into<String> + Clone,
    {
        let content: String = content.into();

        let mut vars = vec![];

        let mut pairing = false;
        let (mut line, mut offset) = (0, 0);
        let mut lpair_offset = (0, 0);

        let mut var = String::new();
        for ele in content.lines().into_iter() {
            line += 1;
            offset = 0;
            let mut skip = false;
            for ele in ele.to_string().as_bytes() {
                offset += 1;

                if skip {
                    skip = false;
                    var.push(*ele as char);
                    continue;
                }

                match *ele as char {
                    '\\' => skip = true,
                    '{' => {
                        pairing = true;
                        lpair_offset.0 = line;
                        lpair_offset.1 = offset;
                    }
                    '}' => {
                        pairing = false;
                        vars.push(var.clone());
                    }
                    _ => {
                        if pairing {
                            var.push(*ele as char);
                        }
                    }
                }
            }
        }

        if pairing == true {
            return Err(anyhow!(format!(
                "Sytax error: found left pair at {}:{} but mission right pair.",
                line, offset
            )));
        }
        Ok(vars)
    }
}

#[cfg(test)]
#[test]
fn extract_vars_test() {
    let content_1 = r"{TEST_VAR}1122333";
    assert_eq!(
        ManifestContent::extract_vars(content_1)
            .unwrap()
            .get(0)
            .unwrap(),
        "TEST_VAR"
    );

    let content_2 = r"{TEST_VAR{";
    assert_eq!(ManifestContent::extract_vars(content_2).is_err(), true);
}
