use std::io::BufReader;
use std::process::{ChildStdout, Stdio};
use std::{
    path::PathBuf,
    process::{Child, Command},
};

#[allow(unused)]
#[derive(Debug, Default)]
pub struct Executor {
    executable: PathBuf,
    args: Vec<String>,
    current_dir: PathBuf,
}

#[allow(unused)]
impl Executor {
    fn new<T0, T1>(executable: T0, args: Vec<T1>) -> Self
    where
        T0: Into<PathBuf> + Clone,
        T1: Into<String> + Clone,
    {
        Self {
            executable: executable.into(),
            args: args.into_iter().map(Into::<String>::into).collect(),
            ..Default::default()
        }
    }

    pub async fn exec(self) -> anyhow::Result<Child> {
        let mut args = vec![self.executable.to_str().unwrap().to_string()];
        args.append(&mut self.args.clone());

        Ok(Command::new("sh")
            .current_dir(self.current_dir)
            .arg("-c")
            .arg(args.join(" "))
            .stdout(Stdio::piped())
            .spawn()?)
    }

    pub fn current_dir<T>(&mut self, dir: T)
    where
        T: Into<PathBuf> + Clone,
    {
        self.current_dir = dir.into()
    }

    pub async fn output_reader(mut child: Child) -> (BufReader<ChildStdout>, Child) {
        let reader = BufReader::new(child.stdout.take().expect("Cannot open output stream"));
        (reader, child)
    }
}

impl<T> From<T> for Executor
where
    T: Into<String>,
{
    fn from(item: T) -> Self {
        let item: String = Into::<String>::into(item.into());

        Executor {
            args: item.split(" ").map(str::to_string).collect(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[test]
fn executor_test() {
    use std::io::{BufRead, BufReader};

    let rt = tokio::runtime::Runtime::new().expect("Cannot initializing Tokio runtime");
    rt.block_on(async {
        for mut command in vec![
            Executor::new("ls", vec!["/"])
                .exec()
                .await
                .expect("Cannot execute command."),
            Executor::from("ls /")
                .exec()
                .await
                .expect("Cannot execute command."),
        ] {
            let reader = BufReader::new(command.stdout.take().expect("Cannot open output stream"));

            let result: Vec<String> = reader
                .lines()
                .into_iter()
                .filter(Result::is_ok)
                .map(Result::unwrap)
                .collect();

            command.wait().expect("Cannot get command exit code");

            assert!(result.contains(&String::from("etc")))
        }
    });
}
