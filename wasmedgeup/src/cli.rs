use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short = 'V', long)]
    pub verbose: bool,

    /// Disable progress output
    #[arg(short, long)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install WasmEdge runtime
    Install {
        /// Version to install (use 'latest' for the latest version)
        version: String,

        /// Installation path
        #[arg(short, long, default_value = "~/.wasmedge")]
        path: PathBuf,

        /// Temporary directory for downloads
        #[arg(short, long, default_value = "/tmp")]
        tmpdir: PathBuf,

        /// Override OS detection
        #[arg(short, long)]
        os: Option<String>,

        /// Override architecture detection
        #[arg(short, long)]
        arch: Option<String>,
    },

    /// List available WasmEdge versions
    List,

    /// Remove WasmEdge installation
    Remove {
        /// Installation path to remove from
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Plugin management commands
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
}

#[derive(Subcommand)]
pub enum PluginCommands {
    /// Install plugins
    Install {
        /// Plugin names to install (can specify version with name@version)
        plugins: Vec<String>,
    },

    /// List available plugins
    List,

    /// Remove plugins
    Remove {
        /// Plugin names to remove (can specify version with name@version)
        plugins: Vec<String>,
    },
}

impl Cli {
    pub fn parse_plugin_name_version(plugin_spec: &str) -> (String, Option<String>) {
        if let Some((name, version)) = plugin_spec.split_once('@') {
            (name.to_string(), Some(version.to_string()))
        } else {
            (plugin_spec.to_string(), None)
        }
    }
} 