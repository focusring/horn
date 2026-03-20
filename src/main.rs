use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use horn::model::{Severity, ValidationReport};
use horn::output::{self, OutputFormat};
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "horn",
    version,
    about = "PDF/UA accessibility checker based on the Matterhorn Protocol"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate PDF files against PDF/UA-1
    Validate {
        /// PDF files or directories to validate
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Write output to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Recursively scan directories for PDF files
        #[arg(short, long)]
        recurse: bool,

        /// Minimum severity to cause a non-zero exit code
        #[arg(long, value_enum, default_value = "error")]
        fail_on: FailOn,
    },

    /// List all available checks
    ListChecks,

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FailOn {
    Error,
    Warning,
    Info,
}

impl FailOn {
    fn min_severity(self) -> Severity {
        match self {
            Self::Error => Severity::Error,
            Self::Warning => Severity::Warning,
            Self::Info => Severity::Info,
        }
    }
}

fn main() -> ExitCode {
    env_logger::init();

    let cli = Cli::parse();

    match run(cli) {
        Ok(compliant) => {
            if compliant {
                ExitCode::from(0)
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<bool> {
    match cli.command {
        Commands::Validate {
            files,
            format,
            output: output_path,
            recurse,
            fail_on,
        } => {
            let pdf_paths = collect_pdf_paths(&files, recurse)?;

            if pdf_paths.is_empty() {
                anyhow::bail!("No PDF files found");
            }

            let show_progress = output_path.is_some() || !io::stderr().is_terminal();
            let report = horn::validate_files_parallel(&pdf_paths, show_progress);

            write_output(&report, format, output_path.as_deref())?;
            Ok(report.is_compliant_at(fail_on.min_severity()))
        }
        Commands::ListChecks => {
            let registry = horn::checks::CheckRegistry::new();
            for check in registry.checks() {
                println!(
                    "{:<20} [checkpoint {:>2}]  {}",
                    check.id(),
                    check.checkpoint(),
                    check.description()
                );
            }
            Ok(true)
        }
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "horn", &mut io::stdout());
            Ok(true)
        }
    }
}

fn collect_pdf_paths(inputs: &[PathBuf], recurse: bool) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    for input in inputs {
        if input.is_file() {
            paths.push(input.clone());
        } else if input.is_dir() {
            if recurse {
                for entry in walkdir::WalkDir::new(input)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(std::result::Result::ok)
                {
                    let path = entry.path();
                    if path.is_file()
                        && path
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
                    {
                        paths.push(path.to_path_buf());
                    }
                }
            } else {
                // Non-recursive: only immediate PDF children
                for entry in std::fs::read_dir(input)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file()
                        && path
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
                    {
                        paths.push(path);
                    }
                }
            }
        } else {
            anyhow::bail!("Path does not exist: {}", input.display());
        }
    }

    paths.sort();
    Ok(paths)
}

fn write_output(
    report: &ValidationReport,
    format: OutputFormat,
    output_path: Option<&std::path::Path>,
) -> Result<()> {
    if let Some(path) = output_path {
        let mut file = std::fs::File::create(path)?;
        output::write_report(report, format, &mut file)?;
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        output::write_report(report, format, &mut handle)?;
        handle.flush()?;
    }
    Ok(())
}
