use std::io::Write;

use eframe::egui::{self, CentralPanel};
use egui::Widget;
use egui_directory_combobox::DirectoryComboBox;

pub struct MyApp {
    combobox: DirectoryComboBox
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            self.combobox.ui(ui)
        });
    }
}

fn enter_path() -> String {
    print!("Enter the path for the combo box demo: ");
    std::io::stdout().flush().unwrap();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn main() {
    let path = enter_path();

    let app = MyApp {
        combobox: DirectoryComboBox::new_from_path(path)
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "egui_directory_combobox demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    ).expect("failed to start eframe");
}