use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};

use crate::error::ShedError;
use crate::explain;
use crate::{Config, OutputFormat, analyze, apply_fixes};

#[derive(Debug, Parser)]
#[command(
    name = "cargo-shed",
    version,
    about = "Find what slows your Rust project down."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(long, value_name = "PATH")]
    manifest_path: Option<Utf8PathBuf>,
    #[arg(long)]
    check: bool,
    #[arg(long, value_name = "RULE_ID", num_args = 0..=1, default_missing_value = "")]
    fix: Option<String>,
    #[arg(long, value_enum, default_value_t = CliOutputFormat::Human)]
    format: CliOutputFormat,
    #[arg(long)]
    no_color: bool,
    #[arg(long)]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    Explain { rule_id: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliOutputFormat {
    Human,
    Json,
}

impl From<CliOutputFormat> for OutputFormat {
    fn from(format: CliOutputFormat) -> Self {
        match format {
            CliOutputFormat::Human => Self::Human,
            CliOutputFormat::Json => Self::Json,
        }
    }
}

pub fn run_from_env() -> Result<u8, ShedError> {
    run(normalized_args(env::args_os()))
}

fn run(args: Vec<OsString>) -> Result<u8, ShedError> {
    let cli = Cli::parse_from(args);

    match cli.command.as_ref() {
        Some(Command::Explain { rule_id }) => {
            let text = explain::rule(rule_id)?;
            println!("{text}");
            Ok(0)
        }
        None => run_report(cli),
    }
}

fn run_report(cli: Cli) -> Result<u8, ShedError> {
    let fix = cli.fix.is_some();
    let selected_rule = cli.fix.as_ref().and_then(|rule| {
        if rule.is_empty() {
            None
        } else {
            Some(rule.to_owned())
        }
    });

    let config = Config {
        manifest_path: cli.manifest_path,
        fix,
        check: cli.check,
        format: cli.format.into(),
        selected_rule,
        no_color: cli.no_color,
        verbose: cli.verbose,
    };

    if config.fix {
        let report = apply_fixes(config)?;
        let human = report.to_human();
        println!("{human}");
        return Ok(if report.failed { 1 } else { 0 });
    }

    let check = config.check;
    let format = config.format;
    let report = analyze(config)?;

    match format {
        OutputFormat::Human => {
            let human = if check {
                report.to_check_human()
            } else {
                report.to_human()
            };
            print!("{human}");
        }
        OutputFormat::Json => {
            let json = report.to_json().map_err(|error| ShedError::Parse {
                path: Utf8PathBuf::from("<report>"),
                message: error.to_string(),
            })?;
            println!("{json}");
        }
    }

    Ok(if check && report.has_ci_failures() {
        1
    } else {
        0
    })
}

fn normalized_args(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut args = args.into_iter().collect::<Vec<_>>();

    if args.get(1).is_some_and(|arg| arg == OsStr::new("shed")) {
        args.remove(1);
    }

    args
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::normalized_args;

    #[test]
    fn removes_cargo_subcommand_name() {
        let args = normalized_args(["cargo-shed", "shed", "--help"].map(OsString::from));
        assert_eq!(args, ["cargo-shed", "--help"].map(OsString::from));
    }

    #[test]
    fn keeps_direct_invocation() {
        let args = normalized_args(["cargo-shed", "--help"].map(OsString::from));
        assert_eq!(args, ["cargo-shed", "--help"].map(OsString::from));
    }
}
