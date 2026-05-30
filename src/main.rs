#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod desktop;

use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 380.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "桌面整理大师",
        options,
        Box::new(|cc| {
            load_chinese_font(&cc.egui_ctx);
            Ok(Box::new(App::default()))
        }),
    )
}

fn load_chinese_font(ctx: &egui::Context) {
    let font_paths = [
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\msyhbd.ttf",
        "C:\\Windows\\Fonts\\simkai.ttf",
        "C:\\Windows\\Fonts\\simfang.ttf",
        "C:\\Windows\\Fonts\\SIMYOU.TTF",
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ];

    for path in &font_paths {
        if let Ok(data) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "chinese".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(data)),
            );
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "chinese".to_owned());
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "chinese".to_owned());
            ctx.set_fonts(fonts);
            return;
        }
    }
}

struct App {
    phase: Phase,
    clean_start: Option<Instant>,
    monitor_active: Option<Arc<AtomicBool>>,
}

#[derive(PartialEq)]
enum Phase {
    Idle,
    Cleaning,
    Cleaned,
}

impl Default for App {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            clean_start: None,
            monitor_active: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);
        self.tick_cleaning();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                self.render_header(ui);
                ui.add_space(30.0);
                self.render_body(ui);
                self.render_footer(ui);
            });
        });

        if self.phase == Phase::Cleaning {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
}

impl App {
    fn apply_theme(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220));
        visuals.widgets.hovered.expansion = 2.0;
        visuals.widgets.active.expansion = 1.0;
        ctx.set_visuals(visuals);
    }

    fn tick_cleaning(&mut self) {
        if self.phase == Phase::Cleaning {
            if let Some(start) = self.clean_start {
                if start.elapsed().as_secs_f32() >= 3.0 {
                    self.phase = Phase::Cleaned;
                    #[cfg(windows)]
                    {
                        // 跨进程 SendMessage 不能阻塞主线程
                        std::thread::spawn(|| {
                            desktop::stack_icons_under_window();
                        });
                    }
                    self.start_monitor();
                }
            }
        }
    }

    fn start_monitor(&mut self) {
        #[cfg(windows)]
        {
            let active = Arc::new(AtomicBool::new(true));
            self.monitor_active = Some(active.clone());
            desktop::start_drag_monitor(active);
        }
    }

    fn stop_monitor(&mut self) {
        if let Some(ref a) = self.monitor_active {
            a.store(false, Ordering::Relaxed);
        }
        self.monitor_active = None;
    }

    fn render_header(&self, ui: &mut egui::Ui) {
        ui.add_space(15.0);
        ui.heading(
            egui::RichText::new("桌面整理大师")
                .size(30.0)
                .color(egui::Color32::from_rgb(80, 180, 255)),
        );
        ui.add_space(4.0);
        ui.separator();
    }

    fn render_body(&mut self, ui: &mut egui::Ui) {
        match self.phase {
            Phase::Idle => {
                ui.add_space(40.0);
                let btn = egui::Button::new(egui::RichText::new("清理桌面").size(34.0))
                    .min_size(egui::Vec2::new(240.0, 100.0))
                    .fill(egui::Color32::from_rgb(60, 130, 210))
                    .corner_radius(egui::CornerRadius::same(12));

                if ui.add(btn).clicked() {
                    self.phase = Phase::Cleaning;
                    self.clean_start = Some(Instant::now());
                }

                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new("一键清理，还你整洁桌面")
                        .size(14.0)
                        .color(egui::Color32::from_rgb(140, 140, 140)),
                );
            }

            Phase::Cleaning => {
                let elapsed = self
                    .clean_start
                    .map(|s| s.elapsed().as_secs_f32().min(3.0))
                    .unwrap_or(0.0);
                let pct = elapsed / 3.0;

                ui.add_space(40.0);
                ui.label(egui::RichText::new("正在清理桌面...").size(20.0));
                ui.add_space(16.0);
                ui.add(
                    egui::ProgressBar::new(pct)
                        .desired_width(320.0)
                        .animate(true)
                        .text(format!("{:.0}%", pct * 100.0)),
                );
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("正在将图标收纳到整理盒中...")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(140, 140, 140)),
                );
            }

            Phase::Cleaned => {
                ui.add_space(50.0);
                ui.label(
                    egui::RichText::new("桌面已整理完毕！")
                        .size(24.0)
                        .color(egui::Color32::from_rgb(80, 255, 100)),
                );
            }
        }
    }

    fn render_footer(&self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new("v1.0 — 专业桌面整理解决方案")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(100, 100, 100)),
            );
        });
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.stop_monitor();
        #[cfg(windows)]
        {
            if self.phase == Phase::Cleaned {
                desktop::restore_desktop();
            }
        }
    }
}
