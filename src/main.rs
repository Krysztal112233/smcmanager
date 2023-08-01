use std::{env, fs, path::PathBuf};

use clap::{arg, builder::ValueParser, ArgAction, Command};
use cmd::CMD;
use human_panic::setup_panic;

mod cmd;
mod executor;
mod manifest;
mod service;
mod template;
mod work;

#[tokio::main]
async fn main() {
    setup_panic!();
    run().await;
}

fn cmd() -> Command {
    let workingdir = workingdir();
    let arg_service = arg!(-s --service ... "Services list").action(ArgAction::Append);
    Command::new(env!("CARGO_PKG_NAME"))
        .help_expected(true)
        .version(env!("CARGO_PKG_VERSION"))
        .about("Simple command line tool for SMC Operator team.")
        .author("Krysztal112233 <suibing112233@outlook.com>")
        .arg_required_else_help(true)
        .args([
            arg!(-w --workingdir "Set the working directory of SMC Manager.")
                .default_value(workingdir)
                .value_parser(ValueParser::string()),
            arg!(-q --quite "Quite run command.").action(ArgAction::SetTrue),
            arg!(-j --json "Output use json format.").action(ArgAction::SetTrue),
        ])
        .subcommand(
            Command::new("list")
                .about("List service.")
                .args([arg!(-s --services "Input services.").action(ArgAction::Append)]),
        )
        .subcommand(
            Command::new("start")
                .about("Start one or more services if it's not running.")
                .arg(arg_service.clone()),
        )
        .subcommand(
            Command::new("stop")
                .about("Stop one or more service if it's running.")
                .arg(arg_service.clone()),
        )
        .subcommand(
            Command::new("status")
                .about("Query one or more service running status")
                .arg(arg_service.clone()),
        )
        .subcommand(
            Command::new("template")
                .about("Templates management.")
                .arg_required_else_help(true)
                .subcommand(Command::new("list").about("List all templates."))
                .subcommand(
                    Command::new("create")
                        .about("Create a template(s).")
                        .args([arg!(-n --name <NAME> "Template name.").action(ArgAction::Append)])
                        .arg_required_else_help(true),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete template(s).")
                        .args([arg!(-n --name <NAME> "Template name.").action(ArgAction::Append)])
                        .arg_required_else_help(true),
                ),
        )
        .subcommand(
            Command::new("create")
                .about("Create a service from template")
                .args([
                    arg!(-t --template <NAME> "Template name"),
                    arg!(-n --name <NAME> "Service name."),
                ]),
        )
        .subcommand(Command::new("delete").about("Delete a service if it's not running."))
        .subcommand(
            Command::new("init").about("Init programs. It will download templates from github."),
        )
}

async fn run() {
    let matches = cmd().get_matches();

    let quite = matches.get_flag("quite");
    let json = matches.get_flag("json");
    let workingdir = matches
        .get_one::<String>("workingdir")
        .expect("Cannot get argument `--workingdir`")
        .clone();

    let (_, sub_matches) = matches.subcommand().unwrap().clone();

    let cmd = CMD {
        quite,
        json,
        workingdir,
        matches: sub_matches.clone(),
    };

    match matches.subcommand() {
        Some(("list", _)) => cmd.list().await,
        Some(("start", _)) => cmd.start().await,
        Some(("stop", _)) => cmd.stop().await,
        Some(("status", _)) => cmd.status().await,
        Some(("template", _)) => cmd.template().await,
        Some(("create", _)) => cmd.create().await,
        Some(("delete", _)) => cmd.delete().await,
        Some(("init", _)) => cmd.init().await,

        _ => {}
    }
}

fn workingdir() -> String {
    env::var("SMC_WORKING_DIR").unwrap_or_else(|_| {
        let path = String::from("/var/smc");
        let path = PathBuf::from(&path);
        if !&path.exists() {
            fs::create_dir_all(&path).expect(r"You don't have permission to access `/var/smc`.");
        }
        String::from(path.to_str().unwrap())
    })
}
