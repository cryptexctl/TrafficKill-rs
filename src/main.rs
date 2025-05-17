mod network;
mod servers;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    #[arg(short, long, default_value_t = false)]
    gui: bool,
}

#[derive(Subcommand)]
enum Commands {
    Traffic,
    Wifikill,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("{}", "TrafficDown Rust".green());
    println!("{}", "Инициализация...".yellow());

    let cli = Cli::parse();

    if cli.gui {
        #[cfg(feature = "gui")]
        {
            println!("{}", "Запуск графического интерфейса...".green());
            ui::gui::run_gui()?;
        }
        #[cfg(not(feature = "gui"))]
        {
            println!("{}", "GUI не скомпилирован. Используйте cargo build --features gui для включения GUI.".red());
            println!("{}", "Проверьте, что проект собран с: cargo build --features gui".yellow());
            return Ok(());
        }
    } else if let Some(command) = cli.command {
        let stresser = network::NetworkStresser::new();
        
        match command {
            Commands::Traffic => {
                stresser.traffic_down().await?;
                
                println!("\n{}", "Нажмите Enter для остановки...".yellow());
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                stresser.stop_network_flood();
            }
            Commands::Wifikill => {
                stresser.scan_and_attack().await?;
            }
        }
    } else {
        let terminal = ui::terminal::TerminalUI::new();
        terminal.run().await?;
    }

    Ok(())
}
