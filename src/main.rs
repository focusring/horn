use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use horn::matterhorn::{CONDITIONS, How};
use horn::model::{Severity, ValidationReport};
use horn::output::{self, OutputFormat, OutputOptions};
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

        /// Text output: also list the Matterhorn conditions that need manual review
        #[arg(long)]
        review: bool,
    },

    /// List all available checks
    ListChecks,

    /// Show Matterhorn Protocol coverage: every failure condition and how Horn covers it
    Coverage {
        /// Output as JSON instead of a table
        #[arg(long)]
        json: bool,
    },

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
            review,
        } => {
            let pdf_paths = collect_pdf_paths(&files, recurse)?;

            if pdf_paths.is_empty() {
                anyhow::bail!("No PDF files found");
            }

            let show_progress = output_path.is_some() || !io::stderr().is_terminal();
            let report = horn::validate_files_parallel(&pdf_paths, show_progress);

            let options = OutputOptions {
                show_review: review,
            };
            write_output(&report, format, options, output_path.as_deref())?;
            Ok(report.is_compliant_at(fail_on.min_severity()))
        }
        Commands::Coverage { json } => {
            print_coverage(json)?;
            Ok(true)
        }
        Commands::ListChecks => {
            let registry = horn::checks::CheckRegistry::new();
            for check in registry.checks() {
                println!(
                    "{:<20} [checkpoint {:>2}]  {}  ({} rules)",
                    check.id(),
                    check.checkpoint(),
                    check.description(),
                    check.rules().len()
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
    options: OutputOptions,
    output_path: Option<&std::path::Path>,
) -> Result<()> {
    if let Some(path) = output_path {
        let mut file = std::fs::File::create(path)?;
        output::write_report_with(report, format, options, &mut file)?;
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        output::write_report_with(report, format, options, &mut handle)?;
        handle.flush()?;
    }
    Ok(())
}

/// Print the Matterhorn Protocol coverage table.
fn print_coverage(json: bool) -> Result<()> {
    let registry = horn::checks::CheckRegistry::new();
    let implemented = registry.implemented_rules();
    let covered = |id: &str| implemented.binary_search(&id).is_ok();

    let mut machine_total = 0usize;
    let mut machine_covered = 0usize;
    let mut human_total = 0usize;
    let mut human_covered = 0usize;
    let mut rows = Vec::new();
    for c in &CONDITIONS {
        let status = match c.how {
            How::Machine => {
                machine_total += 1;
                if covered(c.id) {
                    machine_covered += 1;
                    "machine"
                } else {
                    "missing"
                }
            }
            How::Human => {
                human_total += 1;
                if covered(c.id) {
                    human_covered += 1;
                    "manual review"
                } else {
                    "missing"
                }
            }
            How::None => "no test defined",
        };
        rows.push((c, status));
    }
    let extensions: Vec<&str> = implemented
        .iter()
        .copied()
        .filter(|id| horn::matterhorn::is_extension_rule(id))
        .collect();

    if json {
        let conditions: Vec<serde_json::Value> = rows
            .iter()
            .map(|(c, status)| {
                serde_json::json!({
                    "id": c.id,
                    "checkpoint": c.checkpoint,
                    "checkpoint_name": c.checkpoint_name,
                    "section": c.section,
                    "how": c.how,
                    "status": status,
                    "description": c.description,
                })
            })
            .collect();
        let out = serde_json::json!({
            "protocol": "Matterhorn Protocol 1.1",
            "machine_checkable": { "total": machine_total, "covered": machine_covered },
            "human_judgment": { "total": human_total, "covered": human_covered },
            "extension_rules": extensions,
            "conditions": conditions,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!(
        "Matterhorn Protocol 1.1 coverage — horn {}\n",
        env!("CARGO_PKG_VERSION")
    );
    let mut current_cp = 0u8;
    for (c, status) in &rows {
        if c.checkpoint != current_cp {
            current_cp = c.checkpoint;
            println!("Checkpoint {:02}: {}", c.checkpoint, c.checkpoint_name);
        }
        let how = match c.how {
            How::Machine => "M",
            How::Human => "H",
            How::None => "-",
        };
        println!("  {}  {how}  {:<14} {}", c.id, status, c.description);
    }
    println!(
        "\nMachine-checkable: {machine_covered}/{machine_total} implemented; human-judgment: {human_covered}/{human_total} reported for manual review; {} Horn extension rules ({}).",
        extensions.len(),
        extensions.join(", ")
    );
    Ok(())
}
