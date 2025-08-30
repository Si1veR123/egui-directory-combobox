use std::path::{Path, PathBuf};

use egui::RichText;

pub enum DirectoryNode {
    File(PathBuf),
    Directory(PathBuf, Vec<DirectoryNode>),
}

impl DirectoryNode {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        if path.is_dir() {
            let mut children = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    children.push(DirectoryNode::from_path(entry.path()));
                }
            }
            DirectoryNode::Directory(path, children)
        } else {
            DirectoryNode::File(path)
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            DirectoryNode::File(p) => p,
            DirectoryNode::Directory(p, _) => p,
        }
    }
}

pub struct DirectoryComboBox {
    id: egui::Id,
    selected: Option<PathBuf>,
    roots: Vec<DirectoryNode>,
    max_size: egui::Vec2
}

impl Default for DirectoryComboBox {
    fn default() -> Self {
        Self { selected: None, roots: Vec::new(), id: egui::Id::new("directory_combobox"), max_size: egui::Vec2::new(200.0, f32::INFINITY) }
    }
}

impl DirectoryComboBox {
    /// If `path` is a directory, its children will be the selectable values.
    /// 
    /// If `path` is a file, it will be the only selectable value.
    pub fn new_from_path<P: AsRef<Path>>(path: P) -> Self {
        let root_node = DirectoryNode::from_path(path);

        let roots = match root_node {
            DirectoryNode::Directory(_, children) => children,
            DirectoryNode::File(_) => vec![root_node],
        };

        Self { roots, ..Default::default() }
    }

    /// `paths` will each be a root node in the combo box.
    pub fn new_from_paths<P: AsRef<Path>>(paths: &[P]) -> Self {
        let mut roots = Vec::new();
        for path in paths {
            let root_node = DirectoryNode::from_path(path);
            roots.push(root_node);
        }
        Self { roots, ..Default::default() }
    }

    pub fn new_from_nodes(roots: Vec<DirectoryNode>) -> Self {
        Self { roots, ..Default::default() }
    }

    /// Change the id from the default: "directory_combobox"
    pub fn with_id(mut self, id: egui::Id) -> Self {
        self.id = id;
        self
    }

    /// Change the maximum size of each popup menu.
    pub fn with_max_size(mut self, max_size: egui::Vec2) -> Self {
        self.max_size = max_size;
        self
    }
}

fn nested_combobox_ui(
    ui: &mut egui::Ui,
    nodes: &[DirectoryNode],
    is_root: bool,
    id: egui::Id,
    selected: &mut Option<PathBuf>,
    max_size: egui::Vec2
) {
    if is_root {
        ui.selectable_value(selected, None, "None");
    }
    for node in nodes {
        match node {
            DirectoryNode::File(p) => {
                let file_name = p.file_name().expect("File name should be a full path").to_string_lossy();
                if ui.selectable_value(selected, Some(p.clone()), file_name).clicked() {
                    egui::Popup::close_all(ui.ctx());
                };
            }
            DirectoryNode::Directory(dir_path, children) => {
                if let Some(selected_path) = selected {
                    if selected_path.starts_with(dir_path) {
                        // This directory needs its own combo box as it is
                        // selected or an ancestor of the selected item
                        
                        let right_of_combobox = ui.next_widget_position() + egui::Vec2::new(ui.available_width(), 0.0);
                        let combobox_rect = egui::Rect::from_min_size(
                            right_of_combobox,
                            egui::Vec2::ZERO
                        );
                        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(combobox_rect));
                        nested_combobox_popup_ui(
                            &mut child_ui,
                            children,
                            false,
                            id.with("child"),
                            selected,
                            max_size
                        );
                    }
                }

                ui.selectable_value(
                    selected,
                    Some(dir_path.clone()),
                    RichText::new(
                        dir_path.file_name().expect("Directory name should be a full path").to_string_lossy()
                    ).strong()
                );
            }
        }
    }
}

fn nested_combobox_popup_ui(
    ui: &mut egui::Ui,
    nodes: &[DirectoryNode],
    is_root: bool,
    id: egui::Id,
    selected: &mut Option<PathBuf>,
    max_size: egui::Vec2
) {
    egui::Popup::new(
        id,
        ui.ctx().clone(),
        egui::PopupAnchor::Position(ui.next_widget_position()),
        egui::LayerId::new(egui::Order::Foreground, id.with("popup_layer"))
    )
    .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
    .width(max_size.x)
    .show(|ui| {
        egui::ScrollArea::vertical()
            .max_height(max_size.y)
            .show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                nested_combobox_ui(ui, nodes, is_root, id, selected, max_size)
            })
    });
}

impl egui::Widget for &mut DirectoryComboBox {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        egui::ComboBox::from_id_salt(self.id)
            .width(self.max_size.x)
            .height(self.max_size.y)
            .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
            .selected_text(match &self.selected {
                Some(p) => p.file_name().expect("Selected file name should be a full path").to_string_lossy(),
                None => "Select".into(),
            })
            .show_ui(ui, |ui| {
                nested_combobox_ui(
                    ui,
                    &self.roots,
                    true,
                    self.id.with("child"),
                    &mut self.selected,
                    self.max_size
                )
            }).response
    }
}
