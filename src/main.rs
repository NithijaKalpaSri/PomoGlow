#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod clock;
mod kit;
mod sound;
mod store;
mod timer;
mod tray;

use app::PomoglowApp;
use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut bold_loaded = false;
    if let Ok(reg) = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), Arc::new(FontData::from_owned(reg)));
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "segoe".into());
    }
    if let Ok(b) = std::fs::read("C:\\Windows\\Fonts\\segoeuib.ttf") {
        fonts.font_data.insert("segoe-bold".into(), Arc::new(FontData::from_owned(b)));
        let mut list = vec!["segoe-bold".to_string()];
        list.extend(fonts.families[&FontFamily::Proportional].clone());
        fonts.families.insert(FontFamily::Name("bold".into()), list);
        bold_loaded = true;
    }
    if !bold_loaded {
        let list = fonts.families[&FontFamily::Proportional].clone();
        fonts.families.insert(FontFamily::Name("bold".into()), list);
    }
    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result {
    let mut vp = egui::ViewportBuilder::default();
    if let Ok(img) = image::load_from_memory(include_bytes!("../assets/icon256.png")) {
        let img = img.to_rgba8();
        vp = vp.with_icon(egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() });
    }
    if std::env::var_os("POMOGLOW_PAGE").is_some() {
        vp = vp.with_position([20.0, 4.0]);
    }
    let options = eframe::NativeOptions {
        viewport: vp
            .with_inner_size([1040.0, 640.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Pomoglow"),
        ..Default::default()
    };

    eframe::run_native(
        "Pomoglow",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            let dark = store::SaveData::load().settings.dark;
            kit::set_dark(dark);
            cc.egui_ctx.set_visuals(kit::visuals());
            Ok(Box::new(PomoglowApp::new(cc)))
        }),
    )
}
