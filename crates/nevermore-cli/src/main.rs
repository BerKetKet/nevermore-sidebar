use clap::{Parser, Subcommand};
use nevermore_core::{SidebarEvent, SidebarStateMachine};

#[derive(Parser, Debug)]
#[command(name = "nevermore")]
#[command(author = "bgg")]
#[command(version = "0.1.0")]
#[command(about = "Nevermore Wayland Sidebar CLI controller", long_about = None)]
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
}

fn main() {
    let cli = Cli::parse();

    // Prototype state machine invocation (in later phases, connected to the daemon socket)
    let mut sm = SidebarStateMachine::new();

    match cli.command {
        Commands::Toggle => {
            let next = sm.transition(SidebarEvent::CliToggle);
            println!("Sidebar toggled -> {:?}", next);
        }
        Commands::Show { tab } => {
            let next = sm.transition(SidebarEvent::CliShow { tab: tab.clone() });
            println!(
                "Sidebar show (tab: {}) -> {:?}",
                tab.as_deref().unwrap_or("default"),
                next
            );
        }
        Commands::Hide => {
            let next = sm.transition(SidebarEvent::CliHide);
            println!("Sidebar hidden -> {:?}", next);
        }
        Commands::Pin => {
            let next = sm.transition(SidebarEvent::CliPin);
            println!("Sidebar pin toggled -> {:?}", next);
        }
        Commands::Status => {
            println!("State: {:?}", sm.state());
            println!("Visible: {}", sm.state().is_visible());
            println!("Keyboard Focus: {}", sm.state().takes_keyboard_focus());
            println!("Active Tab: {}", sm.active_tab());
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
    }
}
