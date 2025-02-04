mod cli;
mod downloader;
mod installer;
mod platform;
mod plugin;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, PluginCommands};
use installer::Installer;
use platform::{Architecture, OS, Platform};
use plugin::PluginManager;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging based on verbosity
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else if !cli.quiet {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    }

    match &cli.command {
        Commands::Install { version, path, tmpdir, os, arch } => {
            let platform = match (os, arch) {
                (Some(os_str), Some(arch_str)) => {
                    Platform::new(
                        OS::from_str(os_str)?,
                        Architecture::from_str(arch_str)?,
                    )
                }
                _ => Platform::detect()?,
            };

            let install_path = expand_path(path)?;
            let temp_dir = expand_path(tmpdir)?;

            let installer = Installer::new(install_path, temp_dir, platform.clone());
            
            let version = if version == "latest" {
                "0.14.1".to_string()
            } else {
                version.clone()
            };

            installer.install_runtime(&version).await?;
            println!("Successfully installed WasmEdge {}", version);
        }

        Commands::List => {
            // Implement version listing
            println!("Available versions:");
            println!("0.14.1 <- latest");
            println!("0.14.0");
            println!("0.13.5");
        }

        Commands::Remove { path } => {
            let install_path = expand_path(path)?;
            let platform = Platform::detect()?;
            let installer = Installer::new(
                install_path.clone(),
                PathBuf::from("/tmp"),
                platform,
            );

            installer.remove_runtime().await?;
            println!("Successfully removed WasmEdge from {}", install_path.display());
        }

        Commands::Plugin { command } => {
            let platform = Platform::detect()?;
            log::debug!("Detected platform: {} {}", platform.os, platform.arch);
            
            let plugin_manager = PluginManager::new("0.14.1".to_string(), platform.clone());

            match command {
                PluginCommands::Install { plugins } => {
                    for plugin_spec in plugins {
                        let (name, version) = Cli::parse_plugin_name_version(plugin_spec);
                        log::debug!("Installing plugin {} version {:?}", name, version);
                        plugin_manager.install_plugin(&name, version).await?;
                        println!("Successfully installed plugin {}", name);
                    }
                }

                PluginCommands::List => {
                    log::debug!("Listing available plugins for platform {} {}", platform.os, platform.arch);
                    let plugins = plugin_manager.list_available_plugins().await?;
                    println!("Available plugins:");
                    for (name, version, is_compatible) in plugins {
                        if is_compatible {
                            println!("{} {}", name, version);
                        } else {
                            println!("{} {} [Not compatible with {} {}]", 
                                name, version, platform.os, platform.arch);
                        }
                    }
                }

                PluginCommands::Remove { plugins } => {
                    for plugin_spec in plugins {
                        let (name, version) = Cli::parse_plugin_name_version(plugin_spec);
                        log::debug!("Removing plugin {} version {:?}", name, version);
                        plugin_manager.remove_plugin(&name, version)?;
                        println!("Successfully removed plugin {}", name);
                    }
                }
            }
        }
    }

    Ok(())
}

fn expand_path(path: &PathBuf) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();
    let expanded = if path_str.starts_with('~') {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .join(path_str.strip_prefix("~/").unwrap_or(&path_str))
    } else {
        path.clone()
    };
    Ok(expanded)
}
