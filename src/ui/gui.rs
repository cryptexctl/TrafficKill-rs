use crate::network::NetworkStresser;
use eframe::{egui, App, Frame, CreationContext};
use egui::{Align, Color32, Label, Layout, RichText, Ui, Vec2};
use tokio::runtime::Runtime;
use anyhow::Result;

pub struct TrafficDownApp {
    stresser: NetworkStresser,
    runtime: Runtime,
    eating_traffic: bool,
    killing_wifi: bool,
    show_credits: bool,
    show_changelog: bool,
}

impl TrafficDownApp {
    pub fn new(_cc: &CreationContext) -> Self {
        Self {
            stresser: NetworkStresser::new(),
            runtime: Runtime::new().unwrap(),
            eating_traffic: false,
            killing_wifi: false,
            show_credits: false,
            show_changelog: false,
        }
    }

    fn show_status(&mut self, ui: &mut Ui) {
        let status = self.stresser.status_message.lock().unwrap().clone();
        
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            ui.add_space(20.0);
            ui.add(Label::new(
                RichText::new(status).size(18.0).color(Color32::from_rgb(200, 200, 200))
            ));
            ui.add_space(20.0);
        });
    }

    fn show_buttons(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            let eat_text = if self.eating_traffic { "Остановить" } else { "Есть трафик" };
            let eat_color = if self.eating_traffic { Color32::from_rgb(200, 50, 50) } else { Color32::from_rgb(0, 142, 99) };
            
            if ui.add(egui::Button::new(RichText::new(eat_text).color(Color32::WHITE))
                .fill(eat_color)
                .min_size(Vec2::new(150.0, 40.0)))
                .clicked() 
            {
                self.eating_traffic = !self.eating_traffic;
                
                if self.eating_traffic {
                    let stresser = self.stresser.clone();
                    self.runtime.spawn(async move {
                        let _ = stresser.traffic_down().await;
                    });
                } else {
                    self.stresser.stop_network_flood();
                }
            }
            
            ui.add_space(10.0);
            
            let kill_text = if self.killing_wifi { "Остановить" } else { "Убить интернет" };
            let kill_color = if self.killing_wifi { Color32::from_rgb(200, 50, 50) } else { Color32::from_rgb(0, 142, 99) };
            
            if ui.add(egui::Button::new(RichText::new(kill_text).color(Color32::WHITE))
                .fill(kill_color)
                .min_size(Vec2::new(150.0, 40.0)))
                .clicked() 
            {
                self.killing_wifi = !self.killing_wifi;
                
                if self.killing_wifi {
                    let stresser = self.stresser.clone();
                    self.runtime.spawn(async move {
                        let _ = stresser.scan_and_attack().await;
                    });
                } else {
                    self.stresser.stop_network_flood();
                }
            }
        });
    }

    fn show_bottom_buttons(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Changelog").clicked() {
                    self.show_changelog = true;
                }
                if ui.button("Credits").clicked() {
                    self.show_credits = true;
                }
            });
        });
    }

    fn show_credits_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Credits")
            .fixed_size(Vec2::new(300.0, 200.0))
            .open(&mut self.show_credits)
            .show(ctx, |ui| {
                ui.add(Label::new(RichText::new("TrafficDown Credits").size(16.0).strong()));
                ui.add_space(10.0);
                
                ui.add(Label::new(RichText::new("Main Developer:").size(12.0).strong()));
                ui.add(Label::new("Github @Sonys9\nTikTok @взломщик\nTelegram @freedomleaker2"));
                
                ui.add_space(10.0);
                
                ui.add(Label::new(RichText::new("Multi-threading, Server System & Rewrite to Rust:").size(12.0).strong()));
                ui.add(Label::new("Github @cryptexctl\nTelegram @systemxpore"));
            });
    }

    fn show_changelog_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Changelog")
            .fixed_size(Vec2::new(400.0, 500.0))
            .open(&mut self.show_changelog)
            .show(ctx, |ui| {
                ui.add(Label::new(RichText::new("TrafficDown Changelog").size(16.0).strong()));
                ui.add_space(10.0);
                
                ui.add(Label::new("Версия 1.0.0 (Rust):\n- Полная перепись на Rust\n- Улучшена производительность\n- Оптимизирована работа с сетью"));
            });
    }
}

impl App for TrafficDownApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_status(ui);
            self.show_buttons(ui);
            self.show_bottom_buttons(ui);
        });
        
        if self.show_credits {
            self.show_credits_window(ctx);
        }
        
        if self.show_changelog {
            self.show_changelog_window(ctx);
        }
    }
}

pub fn run_gui() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_resizable(false),
        ..Default::default()
    };
    
    eframe::run_native(
        "TrafficDown | by Sonys9",
        options,
        Box::new(|cc| Box::new(TrafficDownApp::new(cc)))
    ).map_err(|e| anyhow::anyhow!("Ошибка запуска GUI: {:?}", e))?;
    
    Ok(())
} 