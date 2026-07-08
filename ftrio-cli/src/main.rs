//! The `ftrio` CLI: a toggle report (default), plus `export-manifest`, `release-check`, and `lint`.
//!
//! Where the .NET/Python scanners use Roslyn/`ast`, the Rust scanner parses `.rs` files with `syn`.

mod appconfig;
mod conformance;
mod export;
mod lint;
mod release;
mod report;
mod scan;
mod table;
mod util;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use export::ExportOptions;
use lint::LintOptions;
use release::ReleaseOptions;
use report::ReportOptions;

#[derive(Parser)]
#[command(
    name = "ftrio",
    version,
    about = "FtrIO feature-toggle tooling for Rust"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    report: ReportArgs,
}

#[derive(Subcommand)]
enum Command {
    /// Scan for toggle usage and write a manifest JSON.
    #[command(name = "export-manifest")]
    ExportManifest(ExportArgs),
    /// Check that every manifest toggle exists in a target config.
    #[command(name = "release-check")]
    ReleaseCheck(ReleaseArgs),
    /// Report `#[toggle]` functions whose key is missing from `appsettings.json` (FTRIO001).
    Lint(LintArgs),
    /// Resolve one conformance case read as JSON on stdin, printing the outcome as JSON. Hidden: it
    /// is the hook the ftrio-conformance cross-port driver drives, not a user-facing command.
    #[command(name = "conformance-resolve", hide = true)]
    ConformanceResolve,
}

#[derive(Args)]
struct ReportArgs {
    /// Directory (or file) to scan for `.rs` toggle usage. Defaults to the current directory.
    #[arg(long)]
    source: Option<PathBuf>,
    /// Directory or file holding `appsettings*.json`. Defaults to `--source`.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Resolve a single environment using the base+overlay model.
    #[arg(long)]
    env: Option<String>,
    /// Also write a markdown report to this file.
    #[arg(long)]
    markdown: Option<PathBuf>,
    /// Include an Overrides column.
    #[arg(long)]
    show_overrides: bool,
}

#[derive(Args)]
struct ExportArgs {
    #[arg(long)]
    source: Option<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    pretty: bool,
}

#[derive(Args)]
struct ReleaseArgs {
    #[arg(long)]
    manifest: PathBuf,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    config_url: Option<String>,
    #[arg(long)]
    env_name: Option<String>,
    #[arg(long)]
    markdown: Option<PathBuf>,
    #[arg(long)]
    warn_only: bool,
}

#[derive(Args)]
struct LintArgs {
    /// Path to scan. Defaults to the current directory.
    path: Option<PathBuf>,
    #[arg(short = 'v', long)]
    verbose: bool,
}

fn current_dir_or_dot() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Some(Command::ExportManifest(args)) => export::run(ExportOptions {
            source: args.source.unwrap_or_else(current_dir_or_dot),
            output: args.output,
            pretty: args.pretty,
        }),
        Some(Command::ReleaseCheck(args)) => release::run(ReleaseOptions {
            manifest: args.manifest,
            config: args.config,
            config_url: args.config_url,
            environment_name: args.env_name,
            markdown: args.markdown,
            warn_only: args.warn_only,
        }),
        Some(Command::Lint(args)) => lint::run(LintOptions {
            path: args.path.unwrap_or_else(current_dir_or_dot),
            verbose: args.verbose,
        }),
        Some(Command::ConformanceResolve) => conformance::run(),
        None => {
            let source = cli.report.source.unwrap_or_else(current_dir_or_dot);
            report::run(ReportOptions {
                source,
                config: cli.report.config,
                environment: cli.report.env,
                markdown: cli.report.markdown,
                show_overrides: cli.report.show_overrides,
            })
        }
    };

    std::process::exit(exit_code);
}
