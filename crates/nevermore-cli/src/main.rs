use clap::{Parser, Subcommand};
use nevermore_core::{send_ipc_request, IpcRequest, IpcResponse};

#[derive(Parser, Debug)]
#[command(name = "nevermore")]
#[command(author = "bgg")]
#[command(version = "0.1.0")]
#[command(
    about = "Nevermore Wayland Sidebar CLI controller",
    long_about = "Controls and inspects the resident Nevermore Wayland sidebar via local IPC."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
enum Commands {
    /// Toggle sidebar visibility (open if hidden/peek, hide if engaged/pinned)
    Toggle,
    /// Open the sidebar and optionally focus a specific tab
    Show {
        /// Tab name to display (e.g. terminal, tasks, widgets, notifications)
        tab: Option<String>,
    },
    /// Slide out and hide the sidebar
    Hide,
    /// Toggle the pinned state of the sidebar
    Pin,
    /// Output the current sidebar state, active tab, and visibility
    Status,
    /// List active terminal sessions running in the tmux backend
    Sessions,
    /// List active background transient tasks managed by systemd
    Tasks,
    /// Launch a command as a transient background task via systemd
    Run {
        /// Command string to execute
        command: String,
        /// Optional working directory
        #[arg(short, long)]
        dir: Option<String>,
    },
    /// Raise or activate an external application window by desktop entry
    Raise {
        /// Desktop entry name (e.g. org.kde.konsole)
        desktop_entry: String,
    },
    /// Manage visual themes (list or set active theme)
    Theme {
        #[command(subcommand)]
        action: ThemeCommands,
    },
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
enum ThemeCommands {
    /// List all available themes and indicate the active theme
    List,
    /// Set and apply a theme dynamically
    Set {
        /// Name of the theme to set
        name: String,
    },
    /// Validate a theme TOML file for strict key rules, formats, and WCAG contrast
    Check {
        /// Path to theme TOML file
        path: std::path::PathBuf,
        /// Output validation results in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Print expanded theme TOML derived from a [seed] configuration
    Expand {
        /// Path to theme TOML file
        path: std::path::PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    // Local theme validation and expansion don't require resident IPC daemon
    if let Commands::Theme { action } = &cli.command {
        match action {
            ThemeCommands::Check { path, json } => {
                match nevermore_core::theme::check_theme_file(path) {
                    Ok(res) => {
                        if *json {
                            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
                        } else if res.valid {
                            println!("Theme check passed: {}", path.display());
                            println!(
                                "  Contrast text_primary:   {:.2}:1 (>= 4.5:1)",
                                res.contrast_ratios.text_primary_on_bg
                            );
                            println!(
                                "  Contrast text_secondary: {:.2}:1 (>= 3.0:1)",
                                res.contrast_ratios.text_secondary_on_bg_secondary
                            );
                            println!(
                                "  Contrast accent_color:   {:.2}:1 (>= 3.0:1)",
                                res.contrast_ratios.accent_on_bg_tertiary
                            );
                        } else {
                            eprintln!("Theme check failed: {}", path.display());
                            for err in &res.errors {
                                eprintln!("  - error: {}", err);
                            }
                            std::process::exit(1);
                        }
                        return;
                    }
                    Err(e) => {
                        eprintln!("Error checking theme: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            ThemeCommands::Expand { path } => {
                match nevermore_core::theme::expand_theme_file(path) {
                    Ok(expanded) => {
                        println!("{}", expanded);
                        return;
                    }
                    Err(e) => {
                        eprintln!("Error expanding theme: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            _ => {}
        }
    }

    let request = match &cli.command {
        Commands::Toggle => IpcRequest::Toggle,
        Commands::Show { tab } => IpcRequest::Show { tab: tab.clone() },
        Commands::Hide => IpcRequest::Hide,
        Commands::Pin => IpcRequest::Pin,
        Commands::Status => IpcRequest::Status,
        Commands::Sessions => IpcRequest::ListSessions,
        Commands::Tasks => IpcRequest::ListTasks,
        Commands::Run { command, dir } => IpcRequest::LaunchTask {
            command: command.clone(),
            working_dir: dir.clone().unwrap_or_default(),
        },
        Commands::Raise { desktop_entry } => IpcRequest::RaiseApp {
            desktop_entry: desktop_entry.clone(),
        },
        Commands::Theme { action } => match action {
            ThemeCommands::List => IpcRequest::ListThemes,
            ThemeCommands::Set { name } => IpcRequest::SetTheme { name: name.clone() },
            ThemeCommands::Check { .. } | ThemeCommands::Expand { .. } => unreachable!(),
        },
    };

    match send_ipc_request(&request) {
        Ok(IpcResponse::State {
            state,
            active_tab,
            is_visible,
            takes_focus,
        }) => {
            println!("Sidebar State: {:?}", state);
            println!("Active Tab: {}", active_tab);
            println!("Visible: {}", is_visible);
            println!("Keyboard Focus: {}", takes_focus);
        }
        Ok(IpcResponse::Sessions(sessions)) => {
            if sessions.is_empty() {
                println!("No active tmux sessions.");
            } else {
                println!(
                    "{:<10} {:<20} {:<10} {:<8}",
                    "ID", "NAME", "ATTACHED", "WINDOWS"
                );
                println!("{}", "-".repeat(50));
                for s in sessions {
                    println!(
                        "{:<10} {:<20} {:<10} {:<8}",
                        s.id, s.name, s.attached_clients, s.windows_count
                    );
                }
            }
        }
        Ok(IpcResponse::Tasks(tasks)) => {
            if tasks.is_empty() {
                println!("No background tasks found.");
            } else {
                println!("{:<35} {:<15} {:<30}", "UNIT", "STATE", "COMMAND");
                println!("{}", "-".repeat(80));
                for t in tasks {
                    println!("{:<35} {:<15?} {:<30}", t.unit, t.state, t.command);
                }
            }
        }
        Ok(IpcResponse::Logs(logs)) => {
            print!("{}", logs);
        }
        Ok(IpcResponse::Themes { active, available }) => {
            println!("Active Theme: {}", active);
            println!("\nAvailable Themes:");
            for t in available {
                if t == active {
                    println!("  * {} (active)", t);
                } else {
                    println!("    {}", t);
                }
            }
        }
        Ok(IpcResponse::Ok) => {
            println!("Operation completed successfully.");
        }
        Ok(IpcResponse::Error(err)) => {
            eprintln!("Error from daemon: {}", err);
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("Communication error: {}", err);
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(["nevermore", "toggle"]).unwrap();
        assert_eq!(cli.command, Commands::Toggle);

        let cli = Cli::try_parse_from(["nevermore", "show", "tasks"]).unwrap();
        assert_eq!(
            cli.command,
            Commands::Show {
                tab: Some("tasks".to_string())
            }
        );

        let cli = Cli::try_parse_from(["nevermore", "status"]).unwrap();
        assert_eq!(cli.command, Commands::Status);

        let cli = Cli::try_parse_from(["nevermore", "run", "make build", "-d", "/tmp"]).unwrap();
        assert_eq!(
            cli.command,
            Commands::Run {
                command: "make build".to_string(),
                dir: Some("/tmp".to_string()),
            }
        );

        let cli = Cli::try_parse_from(["nevermore", "theme", "list"]).unwrap();
        assert_eq!(
            cli.command,
            Commands::Theme {
                action: ThemeCommands::List
            }
        );

        let cli = Cli::try_parse_from(["nevermore", "theme", "set", "catppuccin-mocha"]).unwrap();
        assert_eq!(
            cli.command,
            Commands::Theme {
                action: ThemeCommands::Set {
                    name: "catppuccin-mocha".to_string()
                }
            }
        );
    }
}
