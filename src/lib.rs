use std::{path::{Path, PathBuf}, sync::Arc};

use egui::RichText;

#[derive(Debug, Clone, PartialEq)]
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

    pub fn find_parent_directory(&self, path: &Path) -> Option<&DirectoryNode> {
        match self {
            DirectoryNode::File(_) => None,
            DirectoryNode::Directory(dir_path, children) => {
                if path.starts_with(dir_path) {
                    for child in children {
                        if let Some(found) = child.find_parent_directory(path) {
                            return Some(found);
                        }
                    }
                    return Some(self);
                }
                None
            }
        }
    }

    pub fn find_node_of_path(&self, path: &Path) -> Option<&DirectoryNode> {
        match self {
            DirectoryNode::File(p) => {
                if p == path {
                    Some(self)
                } else {
                    None
                }
            }
            DirectoryNode::Directory(dir_path, children) => {
                if dir_path == path {
                    return Some(self);
                }
                for child in children {
                    if let Some(found) = child.find_node_of_path(path) {
                        return Some(found);
                    }
                }
                None
            }
        }
    }
}

#[derive(Clone)]
pub struct DirectoryComboBox {
    id: egui::Id,
    selected: Option<PathBuf>,
    roots: Vec<DirectoryNode>,
    max_size: Option<egui::Vec2>,
    wrap_mode: Option<egui::TextWrapMode>,
    show_extensions: bool,
    filter: Option<Arc<Box<dyn Fn(&Path) -> bool>>>,
}

impl Default for DirectoryComboBox {
    fn default() -> Self {
        Self {
            selected: None,
            roots: Vec::new(),
            id: egui::Id::new("directory_combobox"),
            max_size: None,
            wrap_mode: None,
            show_extensions: true,
            filter: None,
        }
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
        self.max_size = Some(max_size);
        self
    }

    /// Change the text wrap mode of the combo box.
    pub fn with_wrap_mode(mut self, wrap_mode: egui::TextWrapMode) -> Self {
        self.wrap_mode = Some(wrap_mode);
        self
    }

    /// Set a filter function to determine which files are shown.
    pub fn with_filter(mut self, filter: Box<dyn Fn(&Path) -> bool>) -> Self {
        self.filter = Some(Arc::new(filter));
        self
    }

    /// Whether to show file extensions in the combo box.
    pub fn show_extensions(mut self, show: bool) -> Self {
        self.show_extensions = show;
        self
    }



    /// Get the currently selected path, if any.
    pub fn selected(&self) -> Option<&Path> {
        self.selected.as_ref().map(|p| p.as_path())
    }

    fn navigate_folder(&mut self, forward: bool) {
        if let Some(selected_path) = &self.selected {
            for root in &self.roots {
                if let Some(parent) = root.find_parent_directory(&selected_path) {
                    if let DirectoryNode::Directory(_p, children) = parent {
                        let mut found_selected = false;

                        let children_iter = if forward {
                            Box::new(children.iter()) as Box<dyn Iterator<Item = &DirectoryNode>>
                        } else {
                            Box::new(children.iter().rev()) as Box<dyn Iterator<Item = &DirectoryNode>>
                        };

                        for child in children_iter {
                            if let DirectoryNode::File(file_path) = child {
                                if file_path == selected_path {
                                    found_selected = true;
                                } else if found_selected {
                                    self.selected = Some(file_path.clone());
                                    return;
                                }
                            }
                        }

                        if found_selected {
                            // Wrap around to the start/end of the list
                            if forward {
                                for child in children {
                                    if let DirectoryNode::File(file_path) = child {
                                        self.selected = Some(file_path.clone());
                                        return;
                                    }
                                }
                            } else {
                                for child in children.iter().rev() {
                                    if let DirectoryNode::File(file_path) = child {
                                        self.selected = Some(file_path.clone());
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// If a file is selected, select the next file in the parent directory, if one exists.
    pub fn select_next_file(&mut self) {
        self.navigate_folder(true);
    }

    /// If a file is selected, select the previous file in the parent directory, if one exists.
    pub fn select_previous_file(&mut self) {
        self.navigate_folder(false);
    }
}

fn nested_combobox_ui(
    ui: &mut egui::Ui,
    nodes: &[DirectoryNode],
    is_root: bool,
    id: egui::Id,
    selected: &mut Option<PathBuf>,
    max_size: Option<egui::Vec2>,
    show_extensions: bool,
    filter: Option<&Arc<Box<dyn Fn(&Path) -> bool>>>,
) {
    if is_root {
        ui.selectable_value(selected, None, "None");
    }
    for node in nodes {
        match node {
            DirectoryNode::File(p) => {
                let file_name = p.file_name().expect("File name should be a full path").to_string_lossy();

                if let Some(filter) = filter {
                    if !filter(p) {
                        continue;
                    }
                }

                let extension = p.extension().and_then(|ext| ext.to_str()).unwrap_or("");
                let mut file_name_str = file_name.as_ref();
                if file_name.ends_with(extension) && !show_extensions {
                    file_name_str = &file_name_str[..file_name_str.len() - extension.len() - 1];
                }

                if ui.selectable_value(selected, Some(p.clone()), file_name_str).clicked() {
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
                            max_size,
                            show_extensions,
                            filter,
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
    max_size: Option<egui::Vec2>,
    show_extensions: bool,
    filter: Option<&Arc<Box<dyn Fn(&Path) -> bool>>>,
) {
    let mut popup = egui::Popup::new(
        id,
        ui.ctx().clone(),
        egui::PopupAnchor::Position(ui.next_widget_position()),
        egui::LayerId::new(egui::Order::Foreground, id.with("popup_layer"))
    ).close_behavior(egui::PopupCloseBehavior::IgnoreClicks).sense(egui::Sense::click());

    if let Some(max_size) = max_size {
        popup = popup.width(max_size.x);
    }

    popup.show(|ui| {
        let mut scroll = egui::ScrollArea::vertical();

        if let Some(max_size) = max_size {
            scroll = scroll.max_height(max_size.y)
        };
        
        scroll.show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            nested_combobox_ui(ui, nodes, is_root, id, selected, max_size, show_extensions, filter);
        })
    });
}

impl egui::Widget for &mut DirectoryComboBox {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let old_value = self.selected.clone();
        let mut cb = egui::ComboBox::from_id_salt(self.id);

        if let Some(max_size) = self.max_size {
            cb = cb.width(max_size.x).height(max_size.y)
        }

        if let Some(wrap_mode) = self.wrap_mode {
            cb = cb.wrap_mode(wrap_mode);
        }

        let cb_response = cb.close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
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
                    self.max_size,
                    self.show_extensions,
                    self.filter.as_ref(),
                )
            }).response;

        let popups_clicked = cb_response.clicked() || self.selected != old_value;
        // There was a click and no popups were clicked -> close all popups
        if ui.ctx().input(|i| i.pointer.any_click()) && !popups_clicked {
            egui::Popup::close_all(ui.ctx());
        }

        cb_response
    }
}
