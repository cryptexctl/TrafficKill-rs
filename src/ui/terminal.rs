use crate::network::{NetworkStresser, print_logo};
use colored::Colorize;
use crossterm::{
    terminal::{Clear, ClearType, size},
    ExecutableCommand,
};
use std::io::{self, Write};
use anyhow::Result;

pub struct TerminalUI {
    stresser: NetworkStresser,
}

impl TerminalUI {
    pub fn new() -> Self {
        Self {
            stresser: NetworkStresser::new(),
        }
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            self.clear_screen()?;
            
            let (width, _) = self.get_terminal_size()?;
            
            print_logo(width);
            
            let functions = vec![
                ("traffic", "начать съедание трафика"),
                ("wifikill", "убить интернет"),
                ("exit", "выход"),
            ];
            
            for (name, description) in &functions {
                let text = format!("[{}] - {}", name.cyan(), description);
                let spaces = " ".repeat((width / 2) - (text.len() / 2) + 6);
                println!("{}{}", spaces, text);
            }
            
            let text = "Введите название функции:\t";
            let spaces = " ".repeat((width / 2) - (text.len() / 2));
            print!("{}{}", spaces, text);
            io::stdout().flush()?;
            
            let mut choice = String::new();
            io::stdin().read_line(&mut choice)?;
            
            match choice.trim().to_lowercase().as_str() {
                "traffic" => {
                    self.stresser.traffic_down().await?;
                    
                    println!("\n{}", "Нажмите ENTER для остановки".yellow());
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    self.stresser.stop_network_flood();
                }
                "wifikill" => {
                    self.stresser.scan_and_attack().await?;
                }
                "exit" => {
                    break;
                }
                _ => {
                    println!("{}", "Неверный выбор!".red());
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        }
        
        Ok(())
    }
    
    fn clear_screen(&self) -> Result<()> {
        io::stdout().execute(Clear(ClearType::All))?;
        Ok(())
    }
    
    fn get_terminal_size(&self) -> Result<(usize, usize)> {
        match size() {
            Ok((w, h)) => Ok((w as usize, h as usize)),
            Err(_) => Ok((80, 24)) // Default size if terminal size cannot be determined
        }
    }
} 