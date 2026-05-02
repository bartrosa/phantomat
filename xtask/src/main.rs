//! Workspace automation (`xtask` binary).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use chrono::Utc;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "xtask", about = "Phantomat development tasks", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Re-render golden PNGs for **this machine’s** GPU backend (`PHANTOMAT_UPDATE_GOLDENS=1`).
    UpdateGoldens {
        /// Short explanation recorded in `tests/golden/UPDATE_LOG.md` (required).
        #[arg(long)]
        reason: String,
    },
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate lives under workspace root")
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::UpdateGoldens { reason } => {
            let root = workspace_root();
            let golden_dir = root.join("crates/phantomat-renderer/tests/golden");
            let log_path = golden_dir.join("UPDATE_LOG.md");

            let status = Command::new("cargo")
                .current_dir(root)
                .args([
                    "test",
                    "-p",
                    "phantomat-renderer",
                    "--release",
                    "--test",
                    "golden",
                ])
                .env("PHANTOMAT_UPDATE_GOLDENS", "1")
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .expect("spawn cargo test");

            assert!(status.success(), "cargo test update-goldens failed");

            let stamp = Utc::now().to_rfc3339();
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .expect("open UPDATE_LOG.md");

            writeln!(
                f,
                "## {stamp}\n\n**reason:** {reason}\n\n(command: `cargo test -p phantomat-renderer --release --test golden` with `PHANTOMAT_UPDATE_GOLDENS=1`)\n"
            )
            .expect("append log");

            eprintln!(
                "Appended entry to {} (reason recorded).",
                log_path.display()
            );
        }
    }
}
