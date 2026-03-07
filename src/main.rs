use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use std::fs;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use std::process;


// ===========
// === Cli ===
// ===========

#[derive(Parser)]
#[command(name = "rust-formatter")]
#[command(about = "A Rust code formatter that respects your line breaks")]
struct Cli {
    /// Files to format. If empty, reads from stdin.
    files: Vec<PathBuf>,
    /// Write formatted output back to files (instead of stdout).
    #[arg(short, long)]
    write: bool,
    /// Check if files are formatted (exit with error if not).
    #[arg(long)]
    check: bool,
}


// ============
// === main ===
// ============

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.files.is_empty() {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source)?;
        let formatted = rust_formatter::formatter::format_source(&source);
        if cli.check {
            if source != formatted {
                process::exit(1);
            }
        } else {
            print!("{formatted}");
        }
    } else {
        let mut any_unformatted = false;
        for path in &cli.files {
            let source = fs::read_to_string(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let formatted = rust_formatter::formatter::format_source(&source);
            if cli.check {
                if source != formatted {
                    eprintln!("{}: not formatted", path.display());
                    any_unformatted = true;
                }
            } else if cli.write {
                if source != formatted {
                    fs::write(path, &formatted)
                        .with_context(|| format!("Failed to write {}", path.display()))?;
                    eprintln!("Formatted {}", path.display());
                }
            } else {
                print!("{formatted}");
            }
        }
        if cli.check && any_unformatted {
            process::exit(1);
        }
    }
    Ok(())
}
