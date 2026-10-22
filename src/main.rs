//! The txt2tex command line.
//!
//! This file is only argument handling and orchestration — every stage it
//! calls lives in the library, so `txt2tex` the binary and `txt2tex` the crate
//! stay the same program.

use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use std::error::Error;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use txt2tex::model::Resume;
use txt2tex::{analyzer, editor, open, parser, pdf, report, template, tui, TEMPLATE};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Parser)]
#[command(
    name = "txt2tex",
    // Read from Cargo.toml rather than written out here. The version used to
    // be a literal in this file and drifted from the real one.
    version,
    about = "Plain text resume → professional PDF",
    after_help = "Run with no command to open the interactive editor."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,

    /// Resume file to work on
    #[arg(
        short,
        long,
        global = true,
        default_value = "resume.txt",
        value_name = "PATH"
    )]
    file: PathBuf,
}

#[derive(Subcommand)]
enum Cmd {
    /// Open the interactive editor (default)
    Tui,
    /// Create a new resume from the built-in template
    New,
    /// Open the resume in $EDITOR, then build the PDF
    Edit,
    /// Build the PDF without opening an editor
    Build(BuildArgs),
    /// Check the resume and print feedback
    Analyze,
}

#[derive(Args, Default)]
struct BuildArgs {
    /// Write the PDF here instead of next to the resume
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Print the generated LaTeX to stdout instead of building a PDF
    #[arg(long)]
    tex: bool,

    /// Leave the finished PDF closed
    #[arg(long)]
    no_open: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("\n  {} {error}\n", "error".red().bold());
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<()> {
    match cli.command.as_ref().unwrap_or(&Cmd::Tui) {
        Cmd::Tui => interactive(&cli.file),
        Cmd::New => new(&cli.file),
        Cmd::Edit => edit(&cli.file),
        Cmd::Build(args) => build(&cli.file, args),
        Cmd::Analyze => analyze(&cli.file),
    }
}

fn interactive(path: &Path) -> Result<()> {
    // ratatui::init panics outright when there is no terminal to take over.
    // A piped or scripted run should get a sentence, not a stack trace.
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("the interactive editor needs a terminal.\n\
                    To build without one:  txt2tex build"
            .into());
    }

    let resume = if path.exists() {
        parser::parse_file(path)?
    } else {
        // Nothing to open, so start from the template rather than dropping the
        // user into an empty form. Not written to disk until they save.
        parser::parse_content(TEMPLATE)
    };
    tui::run(path.to_path_buf(), resume)?;
    Ok(())
}

fn new(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(format!(
            "{} already exists — refusing to overwrite it.\nEdit it with: txt2tex edit",
            path.display()
        )
        .into());
    }
    fs::write(path, TEMPLATE)?;
    report::done(&format!("created {}", path.display()));
    interactive(path)
}

fn edit(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::write(path, TEMPLATE)?;
        report::done(&format!("created {} from the template", path.display()));
    }
    editor::open(path)?;
    build(path, &BuildArgs::default())
}

fn build(path: &Path, args: &BuildArgs) -> Result<()> {
    let resume = read(path)?;

    let missing = resume.missing_required();
    if !missing.is_empty() {
        return Err(format!(
            "{} needs a {} before it can be built.",
            path.display(),
            missing.join(" and ")
        )
        .into());
    }

    let latex = template::render(&resume);

    // stdout, so `txt2tex build --tex > resume.tex` yields just the document.
    if args.tex {
        print!("{latex}");
        return Ok(());
    }

    let output = args
        .output
        .clone()
        .unwrap_or_else(|| path.with_extension("pdf"));

    report::step("typesetting with tectonic");
    pdf::compile(&latex, &output)?;

    let size = fs::metadata(&output).map(|meta| meta.len()).unwrap_or(0);
    report::done(&format!("{} ({})", output.display(), human_size(size)));

    // The PDF is built either way; the analysis is a nudge, not a gate.
    let findings = analyzer::analyze(&resume);
    if !findings.is_empty() {
        let (issues, tips) = analyzer::counts(&findings);
        report::hint(&format!(
            "{} — see them with: txt2tex analyze",
            report::summary(issues, tips)
        ));
    }

    if !args.no_open {
        open::in_viewer(&output);
    }
    Ok(())
}

fn analyze(path: &Path) -> Result<()> {
    report::analysis(&analyzer::analyze(&read(path)?));
    Ok(())
}

fn read(path: &Path) -> Result<Resume> {
    if !path.exists() {
        return Err(format!(
            "no {} in this folder.\nCreate one with: txt2tex new",
            path.display()
        )
        .into());
    }
    Ok(parser::parse_file(path)?)
}

fn human_size(bytes: u64) -> String {
    match bytes {
        0..=1023 => format!("{bytes} B"),
        1024..=1_048_575 => format!("{:.0} KB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}
