use std::{path::{Path, PathBuf}, sync::Arc};

use egui::RichText;
use dunce::canonicalize;

#[derive(Debug, Clone, PartialEq)]
pub enum DirectoryNode {
    File(PathBuf),
    Directory(PathBuf, Vec<DirectoryNode>),
}

impl DirectoryNode {
    pub fn try_from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        let path = canonicalize(path.as_ref()).ok()?;
        std::fs::create_dir_all(&path).ok()?;
        if path.is_dir() {
            let mut children = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    // entry should start with path, else it is probably a symlink which we ignore
                    if entry.path().starts_with(&path) {
                        children.push(DirectoryNode::try_from_path(entry.path())?);
                    }
                }
            }
            Some(DirectoryNode::Directory(path, children))
        } else if path.is_file() {
            Some(DirectoryNode::File(path))
        } else {
            None
        }
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
       Self::try_from_path(&path).unwrap_or_else(|| {
           panic!(
               "Failed to make DirectoryNode from path: {:?}",
               path.as_ref()
           )
       })
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
    pub id: egui::Id,
    selected_path: Option<PathBuf>,
    selected_file: Option<PathBuf>,
    pub roots: Vec<DirectoryNode>,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
    pub wrap_mode: Option<egui::TextWrapMode>,
    pub show_extensions: bool,
    pub filter: Option<Arc<dyn Fn(&Path) -> bool>>,
    pub select_files_only: bool,
    pub back_button: bool
}

impl Default for DirectoryComboBox {
    fn default() -> Self {
        Self {
            selected_path: None,
            selected_file: None,
            roots: Vec::new(),
            id: egui::Id::new("directory_combobox"),
            max_height: None,
            max_width: None,
            wrap_mode: None,
            show_extensions: true,
            filter: None,
            select_files_only: false,
            back_button: true
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

    /// Change the maximum height of each popup menu.
    pub fn with_max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    /// Change the maximum width of each popup menu.
    pub fn with_max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    /// Change the text wrap mode of the combo box.
    pub fn with_wrap_mode(mut self, wrap_mode: egui::TextWrapMode) -> Self {
        self.wrap_mode = Some(wrap_mode);
        self
    }

    /// Set a filter function to determine which files are shown.
    pub fn with_filter(mut self, filter: Arc<dyn Fn(&Path) -> bool>) -> Self {
        self.filter = Some(filter);
        self
    }

    /// If true, only files can be selected. If false, directories can also be selected, default: false
    pub fn select_files_only(mut self, select_files_only: bool) -> Self {
        self.select_files_only = select_files_only;
        self
    }

    /// Whether to show file extensions in the combo box, default: true
    pub fn show_extensions(mut self, show: bool) -> Self {
        self.show_extensions = show;
        self
    }

    /// If `select_files_only` is true, this will return the last selected file, if any.
    /// 
    /// If `select_files_only` is false, this will return the selected path (file or dir), if any.
    pub fn selected(&self) -> Option<&Path> {
        self.selected_file.as_ref().map(|p| p.as_path())
    }

    /// This will always return the selected path, used to display the open popups.
    pub fn selected_path(&self) -> Option<&Path> {
        self.selected_path.as_ref().map(|p| p.as_path())
    }

    /// Add a bacl button to the popup menus to go to the previous directory, default: true
    pub fn with_back_button(mut self, back_button: bool) -> Self {
        self.back_button = back_button;
        self
    }

    fn navigate_nodes(
        nodes: &[DirectoryNode],
        forward: bool,
        filter: Option<&Arc<dyn Fn(&Path) -> bool>>,
        selected_path: &mut Option<PathBuf>,
        selected_file: &mut Option<PathBuf>,
    ) {
        if let Some(selected_file_unwrap) = &selected_file {
            let children_iter = if forward {
                Box::new(nodes.iter()) as Box<dyn Iterator<Item = &DirectoryNode>>
            } else {
                Box::new(nodes.iter().rev()) as Box<dyn Iterator<Item = &DirectoryNode>>
            };

            let mut found_selected = false;
            for child in children_iter {
                if let DirectoryNode::File(file_path) = child {
                    if file_path == selected_file_unwrap {
                        found_selected = true;
                    } else if found_selected && filter.as_ref().map_or(true, |f| f(file_path)) {
                        *selected_path = Some(file_path.clone());
                        *selected_file = Some(file_path.clone());
                        return;
                    }
                }
            }
            if found_selected {
                // Wrap around to the start/end of the list
                if forward {
                    for child in nodes {
                        if let DirectoryNode::File(file_path) = child {
                            if filter.as_ref().map_or(true, |f| f(file_path)) {
                                *selected_path = Some(file_path.clone());
                                *selected_file = Some(file_path.clone());
                            }
                            return;
                        }
                    }
                } else {
                    for child in nodes.iter().rev() {
                        if let DirectoryNode::File(file_path) = child {
                            if filter.as_ref().map_or(true, |f| f(file_path)) {
                                *selected_path = Some(file_path.clone());
                                *selected_file = Some(file_path.clone());
                            }
                            return;
                        }
                    }
                }
            }
        }
    }

    fn navigate_folder(&mut self, forward: bool) {
        if let Some(selected_file) = &self.selected_file {
            for root in &self.roots {
                if root.path() == selected_file {
                    // Selected file is a root
                    Self::navigate_nodes(
                        &self.roots,
                        forward,
                        self.filter.as_ref(),
                        &mut self.selected_path,
                        &mut self.selected_file
                    );
                    return;
                }
            }
            
            for root in &self.roots {
                if let Some(parent) = root.find_parent_directory(&selected_file) {
                    if let DirectoryNode::Directory(_p, children) = parent {
                        Self::navigate_nodes(
                            children,
                            forward,
                            self.filter.as_ref(),
                            &mut self.selected_path,
                            &mut self.selected_file,
                        );
                        return;
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

    /// Set the selected path to `path`.
    /// 
    /// If `select_files_only` is true, `path` must be a file.
    /// 
    /// Setting `path` to `None` will clear the selection.
    pub fn set_selection<P: AsRef<Path>>(&mut self, path: Option<P>) {
        match path {
            Some(p) => {
                let p = match canonicalize(p.as_ref()).ok() {
                    Some(p) => p,
                    None => return,
                };
                if self.select_files_only {
                    if p.is_file() {
                        self.selected_path = Some(p.clone());
                        self.selected_file = Some(p);
                    }
                } else if p.is_file() {
                    self.selected_path = Some(p.clone());
                    self.selected_file = Some(p);
                } else if p.is_dir() {
                    self.selected_path = Some(p);
                }
            }
            None => {
                self.selected_path = None;
                self.selected_file = None;
            }
        }
    }
}

fn nested_combobox_ui(
    ui: &mut egui::Ui,
    nodes: &[DirectoryNode],
    depth: usize,
    id: egui::Id,
    selected_path: &mut Option<PathBuf>,
    max_height: Option<f32>,
    max_width: Option<f32>,
    show_extensions: bool,
    filter: Option<&Arc<dyn Fn(&Path) -> bool>>,
    back_button: bool,
) {
    if depth == 0 {
        ui.selectable_value(selected_path, None, "None");
    } else if back_button {
        if ui.button(RichText::new("Back").underline()).clicked() {
            if let Some(selected_path_unwrap) = selected_path {
                if depth == 1 {
                    // Go to root
                    *selected_path = None;
                } else {
                    if selected_path_unwrap.is_dir() {
                        *selected_path = selected_path_unwrap.parent().map(|p| p.to_path_buf());
                    } else if selected_path_unwrap.is_file() {
                        // Go up two levels
                        *selected_path = selected_path_unwrap.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf());
                    }
                }
            } else {
                *selected_path = None;
            }
        }
    }

    let mut file_shown = false;

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

                file_shown = true;
                if ui.selectable_value(selected_path, Some(p.clone()), file_name_str).clicked() {
                    // TODO: dont close all popups
                    egui::Popup::close_all(ui.ctx());
                };
            }
            DirectoryNode::Directory(dir_path, children) => {
                if let Some(selected_path_unwrap) = selected_path {
                    if selected_path_unwrap.starts_with(dir_path) {
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
                            depth+1,
                            id.with(dir_path),
                            selected_path,
                            max_height,
                            max_width,
                            show_extensions,
                            filter,
                            back_button
                        );
                    }
                }

                file_shown = true;
                ui.selectable_value(
                    selected_path,
                    Some(dir_path.clone()),
                    RichText::new(
                        dir_path.file_name().expect("Directory name should be a full path").to_string_lossy()
                    ).strong()
                );
            }
        }
    }

    if !file_shown {
        ui.label("Empty");
    }
}

fn nested_combobox_popup_ui(
    ui: &mut egui::Ui,
    nodes: &[DirectoryNode],
    depth: usize,
    id: egui::Id,
    selected_path: &mut Option<PathBuf>,
    max_height: Option<f32>,
    max_width: Option<f32>,
    show_extensions: bool,
    filter: Option<&Arc<dyn Fn(&Path) -> bool>>,
    back_button: bool,
) {
    let mut popup = egui::Popup::new(
        id,
        ui.ctx().clone(),
        egui::PopupAnchor::Position(ui.next_widget_position()),
        egui::LayerId::new(egui::Order::Foreground, id.with("popup_layer"))
    )
    .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
    .sense(egui::Sense::click())
    .layout(egui::Layout::top_down_justified(egui::Align::LEFT))
    .gap(0.0)
    .kind(egui::PopupKind::Menu);

    if let Some(max_width) = max_width {
        popup = popup.width(max_width);
    }

    popup.show(|ui| {

        let mut scroll = egui::ScrollArea::vertical();

        if let Some(max_height) = max_height {
            scroll = scroll.max_height(max_height)
        };
        
        scroll.show(ui, |ui| {
            // Make selectable buttons extend the width of the popup
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            nested_combobox_ui(ui, nodes, depth, id, selected_path, max_height, max_width, show_extensions, filter, back_button);
        })
    });
}

impl egui::Widget for &mut DirectoryComboBox {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let old_value = self.selected_path.clone();
        let mut cb = egui::ComboBox::from_id_salt(self.id);

        if let Some(max_height) = self.max_height {
            cb = cb.height(max_height);
        }

        if let Some(max_width) = self.max_width {
            cb = cb.width(max_width);
        }

        if let Some(wrap_mode) = self.wrap_mode {
            cb = cb.wrap_mode(wrap_mode);
        }

        let selected_text_path = if self.select_files_only {
            self.selected_file.as_ref()
        } else {
            self.selected_path.as_ref()
        };

        let cb_response = cb.close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
            .selected_text(match selected_text_path {
                Some(p) => p.file_name().expect("Selected file name should be a full path").to_string_lossy(),
                None => "Select".into(),
            })
            .show_ui(ui, |ui| {
                nested_combobox_ui(
                    ui,
                    &self.roots,
                    0,
                    self.id.with("child"),
                    &mut self.selected_path,
                    self.max_height,
                    self.max_width,
                    self.show_extensions,
                    self.filter.as_ref(),
                    self.back_button
                )
            }).response;

        let popups_clicked = cb_response.clicked() || self.selected_path != old_value;
        // There was a click and no popups were clicked -> close all popups
        if ui.ctx().input(|i| i.pointer.any_click()) && !popups_clicked {
            // ID of the root popup, a bit hacky
            let id_salt = egui::Id::new(self.id);
            let button_id = ui.make_persistent_id(id_salt);
            let popup_id = button_id.with("popup");
            egui::Popup::close_id(ui.ctx(), popup_id);
        }

        // If select_files_only is true, only set selected_file if a file is selected
        // Else, set selected_file to the selected_path
        if self.selected_path != old_value {
            if self.select_files_only {
                if let Some(selected_path) = &self.selected_path {
                    if selected_path.is_file() {
                        self.selected_file = Some(selected_path.clone());
                    }
                } else {
                    self.selected_file = None;
                }
            } else {
                self.selected_file = self.selected_path.clone();
            }
        }

        cb_response
    }
}
