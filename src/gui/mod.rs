mod preview;
mod query;
mod style;

use crate::app::app::App;
use crate::app::run_configuration::RunConfiguration;
use crate::archive::archive_manager::{format_size, DiskInfo};
use crate::data::file_instance::FileInstance;
use iced::widget::{
    button, column, container, image, row, scrollable, stack, text, text_input, Column, Id, Space,
};
use iced::{Element, Font, Length, Task};

fn file_search_id() -> Id {
    Id::new("file-search")
}

fn tag_filter_id() -> Id {
    Id::new("tag-filter")
}

fn add_tag_id() -> Id {
    Id::new("add-tag")
}

/**
 * Launches the iced GUI. Builds the shared App (config + archive) the same
 * way the CLI does, then hands control to the iced event loop.
 *
 * @param config the already-parsed run configuration.
 * @return iced::Result Ok(()) on a clean exit, Err on a windowing failure.
 */
pub fn run(config: RunConfiguration) -> iced::Result {
    // The boot function must implement `Fn`, but it is only ever invoked
    // once; stash the config behind a Mutex so it can be taken on that
    // single call without requiring RunConfiguration to be Clone.
    let config_holder = std::sync::Mutex::new(Some(config));
    iced::application(
        move || {
            let config = config_holder
                .lock()
                .unwrap()
                .take()
                .expect("gui boot function is only called once");
            let app = App::new(config);
            (TagVfsGui::new(app), Task::none())
        },
        TagVfsGui::update,
        TagVfsGui::view,
    )
    .title("File Vault")
    .theme(TagVfsGui::theme)
    .subscription(TagVfsGui::subscription)
    .run()
}

struct TagVfsGui {
    app: App,
    files: Vec<FileInstance>,
    tags: Vec<(String, usize)>,
    selected_tag: Option<String>,
    selected_file: Option<usize>,
    new_tag_input: String,
    status: String,
    preview_image: Option<image::Handle>,
    preview_note: Option<String>,
    file_query: String,
    file_query_error: Option<String>,
    tag_query: String,
    right_panel: RightPanel,
    disk_info: Option<DiskInfo>,
    show_help: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum RightPanel {
    FileDetail,
    Disk,
    Settings,
}

#[derive(Debug, Clone)]
enum Message {
    SelectTag(Option<String>),
    SelectFile(usize),
    NewTagInputChanged(String),
    AddTag,
    RemoveTag(String),
    Import,
    Refresh,
    FileQueryChanged(String),
    TagQueryChanged(String),
    OpenFile,
    RemoveFile,
    ExportArchive,
    MergeArchive,
    FlushAll,
    DestroyAll,
    ShowDisk,
    ShowSettings,
    FocusFileSearch,
    FocusTagFilter,
    FocusAddTagInput,
    MoveFileSelection(i32),
    MoveTagSelection(i32),
    RemoveTypedTag,
    EscapeBack,
    Quit,
    ToggleHelp,
}

impl TagVfsGui {
    fn new(mut app: App) -> Self {
        let mut tags = app.am().list_tags_data().unwrap_or_default();
        sort_tags_by_count(&mut tags);
        let files = app.am().list_files_data(Vec::new()).unwrap_or_default();
        TagVfsGui {
            app,
            files,
            tags,
            selected_tag: None,
            selected_file: None,
            new_tag_input: String::new(),
            status: String::new(),
            preview_image: None,
            preview_note: None,
            file_query: String::new(),
            file_query_error: None,
            tag_query: String::new(),
            right_panel: RightPanel::FileDetail,
            disk_info: None,
            show_help: false,
        }
    }

    /**
     * Reloads the file list (respecting the current tag filter) and the tag
     * sidebar from the archive. Clears the file selection if it is now
     * out of range.
     */
    fn theme(&self) -> iced::Theme {
        style::theme()
    }

    /**
     * Maps raw key presses to app-level shortcuts. Every shortcut except
     * plain Up/Down and Escape requires the platform "command" modifier
     * (Cmd on macOS, Ctrl elsewhere) so it never collides with typing in a
     * focused text field.
     */
    fn subscription(&self) -> iced::Subscription<Message> {
        iced::keyboard::listen().filter_map(|event| {
            use iced::keyboard::key::Named;

            let iced::keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
                return None;
            };

            let cmd = modifiers.command();
            let shift = modifiers.shift();

            match key.as_ref() {
                iced::keyboard::Key::Named(Named::Escape) => Some(Message::EscapeBack),
                iced::keyboard::Key::Named(Named::ArrowUp) if cmd => Some(Message::MoveTagSelection(-1)),
                iced::keyboard::Key::Named(Named::ArrowDown) if cmd => Some(Message::MoveTagSelection(1)),
                iced::keyboard::Key::Named(Named::ArrowUp) => Some(Message::MoveFileSelection(-1)),
                iced::keyboard::Key::Named(Named::ArrowDown) => Some(Message::MoveFileSelection(1)),
                iced::keyboard::Key::Named(Named::Backspace) if cmd && shift => Some(Message::RemoveTypedTag),
                iced::keyboard::Key::Named(Named::Backspace) if cmd => Some(Message::RemoveFile),
                iced::keyboard::Key::Character("i") if cmd => Some(Message::Import),
                iced::keyboard::Key::Character("o") if cmd => Some(Message::OpenFile),
                iced::keyboard::Key::Character("r") if cmd => Some(Message::Refresh),
                iced::keyboard::Key::Character("t") if cmd => Some(Message::FocusAddTagInput),
                iced::keyboard::Key::Character("f") if cmd && shift => Some(Message::FocusTagFilter),
                iced::keyboard::Key::Character("f") if cmd => Some(Message::FocusFileSearch),
                iced::keyboard::Key::Character("d") if cmd => Some(Message::ShowDisk),
                iced::keyboard::Key::Character(",") if cmd => Some(Message::ShowSettings),
                iced::keyboard::Key::Character("m") if cmd => Some(Message::MergeArchive),
                iced::keyboard::Key::Character("e") if cmd => Some(Message::ExportArchive),
                iced::keyboard::Key::Character("s") if cmd && shift => Some(Message::DestroyAll),
                iced::keyboard::Key::Character("s") if cmd => Some(Message::FlushAll),
                iced::keyboard::Key::Character("q") if cmd => Some(Message::Quit),
                iced::keyboard::Key::Character("/") if cmd => Some(Message::ToggleHelp),
                _ => None,
            }
        })
    }

    fn refresh(&mut self) {
        let filter: Vec<String> = self.selected_tag.clone().into_iter().collect();
        let mut files = self.app.am().list_files_data(filter).unwrap_or_default();
        self.tags = self.app.am().list_tags_data().unwrap_or_default();
        sort_tags_by_count(&mut self.tags);

        self.file_query_error = None;
        match query::parse(&self.file_query) {
            Ok(Some(expr)) => files.retain(|file| query::matches(&expr, file)),
            Ok(None) => {}
            Err(e) => self.file_query_error = Some(e),
        }
        self.files = files;

        if let Some(index) = self.selected_file {
            if index >= self.files.len() {
                self.selected_file = None;
            }
        }
        self.refresh_preview();
    }

    /**
     * Loads a preview for the currently selected file: an image is shown
     * directly; a video's first frame is extracted with the system
     * `ffmpeg` binary, if present. Anything else (or a failed extraction)
     * clears the image and sets an explanatory note instead. Runs the
     * ffmpeg subprocess synchronously — acceptable for a single selected
     * file, matching the rest of this GUI's synchronous archive access.
     */
    fn refresh_preview(&mut self) {
        self.preview_image = None;
        self.preview_note = None;

        let Some(name) = self
            .selected_file
            .and_then(|index| self.files.get(index))
            .map(|file| file.name.clone())
        else {
            return;
        };

        match preview::kind_for(&name) {
            preview::Kind::Image => match self.app.am().read_file_bytes(name) {
                Ok(bytes) => self.preview_image = Some(image::Handle::from_bytes(bytes)),
                Err(_) => self.preview_note = Some("Couldn't load image preview.".to_string()),
            },
            preview::Kind::Video => {
                let ext = std::path::Path::new(&name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string();
                match self.app.am().read_file_bytes(name) {
                    Ok(bytes) => match preview::video_thumbnail(&bytes, &ext) {
                        Some(thumb) => self.preview_image = Some(image::Handle::from_bytes(thumb)),
                        None => {
                            self.preview_note = Some(
                                "No thumbnail available. Install ffmpeg to preview videos."
                                    .to_string(),
                            )
                        }
                    },
                    Err(_) => self.preview_note = Some("Couldn't load video data.".to_string()),
                }
            }
            preview::Kind::Unsupported => {}
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectTag(tag) => {
                self.selected_tag = tag;
                self.selected_file = None;
                self.refresh();
            }
            Message::SelectFile(index) => {
                self.selected_file = Some(index);
                self.right_panel = RightPanel::FileDetail;
                self.refresh_preview();
            }
            Message::NewTagInputChanged(value) => {
                self.new_tag_input = value;
            }
            Message::AddTag => {
                let tag = self.new_tag_input.trim().to_string();
                if let (Some(index), false) = (self.selected_file, tag.is_empty()) {
                    if let Some(file) = self.files.get(index) {
                        let name = file.name.clone();
                        match self.app.am().add_tags(vec![tag], vec![name], Vec::new()) {
                            Ok(()) => {
                                self.new_tag_input.clear();
                                self.status.clear();
                                self.refresh();
                            }
                            Err(e) => self.status = format!("Couldn't add tag: {}", e),
                        }
                    }
                }
            }
            Message::RemoveTag(tag) => {
                if let Some(index) = self.selected_file {
                    if let Some(file) = self.files.get(index) {
                        let name = file.name.clone();
                        match self.app.am().remove_tags(vec![tag], vec![name], Vec::new()) {
                            Ok(()) => {
                                self.status.clear();
                                self.refresh();
                            }
                            Err(e) => self.status = format!("Couldn't remove tag: {}", e),
                        }
                    }
                }
            }
            Message::Import => {
                if let Some(paths) = rfd::FileDialog::new().pick_files() {
                    let paths: Vec<String> = paths
                        .into_iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    match self.app.am().import_files(paths, false) {
                        Ok(()) => {
                            self.status.clear();
                            self.refresh();
                        }
                        Err(e) => self.status = format!("Couldn't import: {}", e),
                    }
                }
            }
            Message::Refresh => {
                self.refresh();
            }
            Message::FileQueryChanged(value) => {
                self.file_query = value;
                self.refresh();
            }
            Message::TagQueryChanged(value) => {
                self.tag_query = value;
            }
            Message::OpenFile => {
                if let Some(file) = self.selected_file.and_then(|index| self.files.get(index)) {
                    let name = file.name.clone();
                    match self.app.am().open(name) {
                        Ok(()) => self.status.clear(),
                        Err(e) => self.status = format!("Couldn't open file: {}", e),
                    }
                }
            }
            Message::RemoveFile => {
                if let Some(file) = self.selected_file.and_then(|index| self.files.get(index)) {
                    let name = file.name.clone();
                    match self.app.am().remove(vec![name], Vec::new()) {
                        Ok(()) => {
                            self.selected_file = None;
                            self.status.clear();
                            self.refresh();
                        }
                        Err(e) => self.status = format!("Couldn't remove file: {}", e),
                    }
                }
            }
            Message::ExportArchive => {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    let destination = folder.to_string_lossy().to_string();
                    match self.app.am().expand(destination) {
                        Ok(()) => self.status.clear(),
                        Err(e) => self.status = format!("Couldn't export archive: {}", e),
                    }
                }
            }
            Message::MergeArchive => {
                if let Some(path) = rfd::FileDialog::new().add_filter("Archive", &["dat"]).pick_file() {
                    let path = path.to_string_lossy().to_string();
                    match self.app.am().merge(path) {
                        Ok(()) => {
                            self.status.clear();
                            self.refresh();
                        }
                        Err(e) => self.status = format!("Couldn't merge archive: {}", e),
                    }
                }
            }
            Message::FlushAll => match self.app.am().flush_all() {
                Ok(()) => self.status.clear(),
                Err(e) => self.status = format!("Couldn't flush cached files: {}", e),
            },
            Message::DestroyAll => match self.app.am().destroy_all() {
                Ok(()) => self.status.clear(),
                Err(e) => self.status = format!("Couldn't clear cached files: {}", e),
            },
            Message::ShowDisk => {
                self.disk_info = self.app.am().disk_info_data().ok();
                self.right_panel = RightPanel::Disk;
            }
            Message::ShowSettings => {
                self.right_panel = RightPanel::Settings;
            }
            Message::FocusFileSearch => return iced::widget::operation::focus(file_search_id()),
            Message::FocusTagFilter => return iced::widget::operation::focus(tag_filter_id()),
            Message::FocusAddTagInput => return iced::widget::operation::focus(add_tag_id()),
            Message::MoveFileSelection(delta) => {
                if !self.files.is_empty() {
                    let len = self.files.len() as i32;
                    let current = self.selected_file.map(|i| i as i32).unwrap_or(-1);
                    let next = (current + delta).rem_euclid(len);
                    self.selected_file = Some(next as usize);
                    self.right_panel = RightPanel::FileDetail;
                    self.refresh_preview();
                }
            }
            Message::MoveTagSelection(delta) => {
                let needle = self.tag_query.trim().to_lowercase();
                let mut names: Vec<Option<String>> = vec![None];
                for (name, _) in &self.tags {
                    if needle.is_empty() || name.to_lowercase().contains(&needle) {
                        names.push(Some(name.clone()));
                    }
                }
                let len = names.len() as i32;
                let current = names
                    .iter()
                    .position(|n| n.as_deref() == self.selected_tag.as_deref())
                    .unwrap_or(0) as i32;
                let next = (current + delta).rem_euclid(len);
                self.selected_tag = names[next as usize].clone();
                self.selected_file = None;
                self.refresh();
            }
            Message::RemoveTypedTag => {
                let tag = self.new_tag_input.trim().to_string();
                if !tag.is_empty() {
                    if let Some(file) = self.selected_file.and_then(|index| self.files.get(index)) {
                        let name = file.name.clone();
                        match self.app.am().remove_tags(vec![tag], vec![name], Vec::new()) {
                            Ok(()) => {
                                self.new_tag_input.clear();
                                self.status.clear();
                                self.refresh();
                            }
                            Err(e) => self.status = format!("Couldn't remove tag: {}", e),
                        }
                    }
                }
            }
            Message::EscapeBack => {
                if self.show_help {
                    self.show_help = false;
                } else if self.right_panel != RightPanel::FileDetail {
                    self.right_panel = RightPanel::FileDetail;
                } else if !self.file_query.is_empty() {
                    self.file_query.clear();
                    self.refresh();
                } else if self.selected_tag.is_some() {
                    self.selected_tag = None;
                    self.selected_file = None;
                    self.refresh();
                }
            }
            Message::Quit => return iced::window::latest().and_then(iced::window::close),
            Message::ToggleHelp => {
                self.show_help = !self.show_help;
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let search = text_input(
            "Search… file:report.pdf and (tag:work or tag:draft)",
            &self.file_query,
        )
        .id(file_search_id())
        .style(style::text_input_style)
        .on_input(Message::FileQueryChanged)
        .padding(style::SPACE_SM)
        .width(Length::FillPortion(4));

        let header = container(
            row![
                text("File Vault").size(18).font(Font {
                    weight: iced::font::Weight::Semibold,
                    ..Font::DEFAULT
                }),
                Space::new().width(Length::FillPortion(1)),
                search,
                Space::new().width(Length::FillPortion(1)),
                button(text("Import").size(14))
                    .style(style::primary_button)
                    .on_press(Message::Import),
                button(text("Refresh").size(14))
                    .style(style::ghost_button)
                    .on_press(Message::Refresh),
                button(text("?").size(14))
                    .style(style::ghost_button)
                    .on_press(Message::ToggleHelp),
            ]
            .spacing(style::SPACE_SM)
            .align_y(iced::Alignment::Center),
        )
        .style(style::header)
        .padding([style::SPACE_MD, style::SPACE_LG])
        .width(Length::Fill);

        let toolbar = container(
            row![
                button(text("Disk").size(13))
                    .style(style::ghost_button)
                    .on_press(Message::ShowDisk),
                button(text("Settings").size(13))
                    .style(style::ghost_button)
                    .on_press(Message::ShowSettings),
                Space::new().width(Length::Fill),
                button(text("Flush all").size(13))
                    .style(style::ghost_button)
                    .on_press(Message::FlushAll),
                button(text("Discard cache").size(13))
                    .style(style::ghost_button)
                    .on_press(Message::DestroyAll),
                button(text("Merge…").size(13))
                    .style(style::ghost_button)
                    .on_press(Message::MergeArchive),
                button(text("Export…").size(13))
                    .style(style::ghost_button)
                    .on_press(Message::ExportArchive),
            ]
            .spacing(style::SPACE_SM)
            .align_y(iced::Alignment::Center),
        )
        .style(style::header)
        .padding([style::SPACE_XS, style::SPACE_LG])
        .width(Length::Fill);

        let content = row![
            container(self.view_sidebar())
                .width(Length::FillPortion(2))
                .style(style::sidebar)
                .padding(style::SPACE_MD),
            container(self.view_file_list())
                .width(Length::FillPortion(4))
                .padding(style::SPACE_MD),
            container(match self.right_panel {
                RightPanel::FileDetail => self.view_detail(),
                RightPanel::Disk => self.view_disk(),
                RightPanel::Settings => self.view_settings(),
            })
            .width(Length::FillPortion(2))
            .style(style::panel)
            .padding(style::SPACE_LG),
        ]
        .spacing(style::SPACE_MD)
        .padding(style::SPACE_MD)
        .height(Length::Fill);

        let mut layout = column![header, toolbar, content]
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(error) = &self.file_query_error {
            layout = layout.push(
                container(
                    text(format!("Search: {}", error))
                        .size(12)
                        .color(style::DANGER),
                )
                .padding([style::SPACE_XS, style::SPACE_LG]),
            );
        }

        if !self.status.is_empty() {
            layout = layout.push(
                container(text(&self.status).size(13).color(style::DANGER))
                    .padding([style::SPACE_XS, style::SPACE_LG]),
            );
        }

        let base: Element<'_, Message> = container(layout)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(style::BG)),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        if self.show_help {
            stack![base, self.view_help_modal()].into()
        } else {
            base
        }
    }

    fn view_help_modal(&self) -> Element<'_, Message> {
        const SHORTCUTS: &[(&str, &str)] = &[
            ("↑ / ↓", "Move selection in the file list"),
            ("Cmd+↑ / Cmd+↓", "Move selection in the tag sidebar"),
            ("Cmd+F", "Focus the file search bar"),
            ("Cmd+Shift+F", "Focus the tag filter box"),
            ("Cmd+T", "Focus the add-a-tag box"),
            ("Cmd+Shift+Backspace", "Remove the typed tag from the selected file"),
            ("Cmd+I", "Import files"),
            ("Cmd+O", "Open the selected file"),
            ("Cmd+Backspace", "Remove the selected file"),
            ("Cmd+R", "Refresh"),
            ("Cmd+D", "Show disk usage"),
            ("Cmd+,", "Show settings"),
            ("Cmd+M", "Merge an archive"),
            ("Cmd+E", "Export the archive"),
            ("Cmd+S", "Flush all cached changes"),
            ("Cmd+Shift+S", "Discard all cached changes"),
            ("Cmd+Q", "Quit"),
            ("Cmd+/", "Show/hide this help"),
            ("Esc", "Go back / close this help"),
        ];

        let mut rows = Column::new().spacing(style::SPACE_SM);
        for (key, action) in SHORTCUTS {
            rows = rows.push(
                row![
                    container(text(*key).size(12).color(style::ACCENT))
                        .width(Length::Fixed(170.0)),
                    text(*action).size(13).color(style::TEXT_PRIMARY),
                ]
                .align_y(iced::Alignment::Center),
            );
        }

        let card = container(
            column![
                row![
                    text("Keyboard shortcuts").size(18).font(Font {
                        weight: iced::font::Weight::Semibold,
                        ..Font::DEFAULT
                    }),
                    Space::new().width(Length::Fill),
                    button(text("Close").size(13))
                        .style(style::ghost_button)
                        .on_press(Message::ToggleHelp),
                ]
                .align_y(iced::Alignment::Center),
                scrollable(rows).height(Length::Fixed(360.0)),
            ]
            .spacing(style::SPACE_MD),
        )
        .style(style::panel)
        .padding(style::SPACE_LG)
        .width(Length::Fixed(480.0));

        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.55,
                })),
                ..Default::default()
            })
            .into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let mut list = Column::new().spacing(style::SPACE_XS);
        list = list.push(section_label("TAGS"));
        list = list.push(
            text_input("Filter tags…", &self.tag_query)
                .id(tag_filter_id())
                .style(style::text_input_style)
                .on_input(Message::TagQueryChanged)
                .padding(style::SPACE_SM)
                .size(13),
        );
        list = list.push(nav_row(
            "All files",
            self.files.len(),
            self.selected_tag.is_none(),
            Message::SelectTag(None),
        ));

        let needle = self.tag_query.trim().to_lowercase();
        for (name, count) in &self.tags {
            if !needle.is_empty() && !name.to_lowercase().contains(&needle) {
                continue;
            }
            let selected = self.selected_tag.as_deref() == Some(name.as_str());
            list = list.push(nav_row(
                name,
                *count,
                selected,
                Message::SelectTag(Some(name.clone())),
            ));
        }
        scrollable(container(list).padding(scrollbar_clearance()))
            .height(Length::Fill)
            .into()
    }

    fn view_file_list(&self) -> Element<'_, Message> {
        let heading = row![text(format!("FILES · {}", self.files.len()))
            .size(12)
            .color(style::TEXT_MUTED),]
        .padding([0.0, style::SPACE_XS]);

        let mut list = Column::new().spacing(style::SPACE_SM);
        if self.files.is_empty() {
            list = list.push(
                container(
                    text("No files yet. Use Import to add some.")
                        .size(13)
                        .color(style::TEXT_MUTED),
                )
                .padding(style::SPACE_LG),
            );
        }
        for (index, file) in self.files.iter().enumerate() {
            let selected = self.selected_file == Some(index);
            let row_content = row![
                column![
                    text(&file.name).size(14),
                    text(format!(
                        "{} tag{}",
                        file.tags.len(),
                        if file.tags.len() == 1 { "" } else { "s" }
                    ))
                    .size(12)
                    .color(if selected {
                        style::ACCENT_DIM
                    } else {
                        style::TEXT_MUTED
                    }),
                ]
                .spacing(2)
                .width(Length::Fill),
                text(file.get_formatted_size())
                    .size(13)
                    .color(style::TEXT_MUTED),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(style::SPACE_SM);

            list = list.push(
                button(row_content)
                    .style(style::file_row(selected))
                    .padding([style::SPACE_SM, style::SPACE_MD])
                    .width(Length::Fill)
                    .on_press(Message::SelectFile(index)),
            );
        }

        let scrollable_list =
            scrollable(container(list).padding(scrollbar_clearance())).height(Length::Fill);

        column![heading, scrollable_list]
            .spacing(style::SPACE_SM)
            .into()
    }

    fn view_detail(&self) -> Element<'_, Message> {
        let file = self.selected_file.and_then(|index| self.files.get(index));
        match file {
            None => container(
                text("Select a file to see its tags.")
                    .size(14)
                    .color(style::TEXT_MUTED),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
            Some(file) => {
                let mut sorted_tags: Vec<&String> = file.tags.iter().collect();
                sorted_tags.sort();
                let chip_elements: Vec<Element<'_, Message>> =
                    sorted_tags.into_iter().map(|tag| tag_chip(tag)).collect();

                let tags_section: Element<'_, Message> = if chip_elements.is_empty() {
                    text("No tags yet.")
                        .size(13)
                        .color(style::TEXT_MUTED)
                        .into()
                } else {
                    row(chip_elements).spacing(style::SPACE_SM).wrap().into()
                };

                let mut items: Vec<Element<'_, Message>> = vec![
                    text(&file.name)
                        .size(18)
                        .font(Font {
                            weight: iced::font::Weight::Semibold,
                            ..Font::DEFAULT
                        })
                        .into(),
                    text(format!(
                        "{} · {}",
                        file.get_formatted_size(),
                        file_type_label(file)
                    ))
                    .size(13)
                    .color(style::TEXT_MUTED)
                    .into(),
                    row![
                        button(text("Open").size(13))
                            .style(style::ghost_button)
                            .padding([style::SPACE_XS, style::SPACE_MD])
                            .on_press(Message::OpenFile),
                        button(text("Remove file").size(13))
                            .style(style::danger_button)
                            .padding([style::SPACE_XS, style::SPACE_MD])
                            .on_press(Message::RemoveFile),
                    ]
                    .spacing(style::SPACE_SM)
                    .into(),
                ];

                if let Some(handle) = &self.preview_image {
                    items.push(
                        container(
                            image(handle.clone())
                                .content_fit(iced::ContentFit::Contain)
                                .width(Length::Fill),
                        )
                        .width(Length::Fill)
                        .height(Length::Fixed(200.0))
                        .padding(style::SPACE_XS)
                        .style(style::preview_frame)
                        .into(),
                    );
                } else if let Some(note) = &self.preview_note {
                    items.push(
                        container(text(note).size(12).color(style::TEXT_MUTED))
                            .width(Length::Fill)
                            .padding(style::SPACE_MD)
                            .style(style::preview_frame)
                            .into(),
                    );
                }

                items.push(section_label("TAGS"));
                items.push(tags_section);
                items.push(
                    row![
                        text_input("add a tag…", &self.new_tag_input)
                            .id(add_tag_id())
                            .style(style::text_input_style)
                            .on_input(Message::NewTagInputChanged)
                            .on_submit(Message::AddTag)
                            .padding(style::SPACE_SM),
                        button(text("Add").size(13))
                            .style(style::primary_button)
                            .padding([style::SPACE_SM, style::SPACE_MD])
                            .on_press(Message::AddTag),
                    ]
                    .spacing(style::SPACE_SM)
                    .align_y(iced::Alignment::Center)
                    .into(),
                );

                column(items).spacing(style::SPACE_MD).into()
            }
        }
    }

    fn view_disk(&self) -> Element<'_, Message> {
        let Some(info) = &self.disk_info else {
            return text("Disk info unavailable.")
                .size(13)
                .color(style::TEXT_MUTED)
                .into();
        };

        let items: Vec<Element<'_, Message>> = vec![
            text("Disk usage")
                .size(18)
                .font(Font {
                    weight: iced::font::Weight::Semibold,
                    ..Font::DEFAULT
                })
                .into(),
            text(format!("Archive size: {}", format_size(info.archive_total_bytes)))
                .size(13)
                .color(style::TEXT_MUTED)
                .into(),
            text(format!("Files: {} / {}", info.files_used, info.files_total))
                .size(13)
                .color(style::TEXT_MUTED)
                .into(),
            text(format!("Tags: {} / {}", info.tags_used, info.tags_total))
                .size(13)
                .color(style::TEXT_MUTED)
                .into(),
            section_label("SECTIONS"),
            section_usage_row("S1 file directory", info.s1_used_bytes, info.s1_total_bytes),
            section_usage_row("S2 tag directory", info.s2_used_bytes, info.s2_total_bytes),
            section_usage_row("S3 tag lookup", info.s3_used_bytes, info.s3_total_bytes),
            section_usage_row("S4 file storage", info.s4_used_bytes, info.s4_total_bytes),
        ];

        column(items).spacing(style::SPACE_MD).into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        column![
            text("Settings").size(18).font(Font {
                weight: iced::font::Weight::Semibold,
                ..Font::DEFAULT
            }),
            text(self.app.config.to_string())
                .size(12)
                .color(style::TEXT_MUTED),
        ]
        .spacing(style::SPACE_MD)
        .into()
    }
}

/**
 * Sorts tags by file count, most-tagged first. Ties break alphabetically
 * so the sidebar order stays stable.
 */
fn sort_tags_by_count(tags: &mut [(String, usize)]) {
    tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
}

/**
 * Right padding to apply inside a scrollable's content, so its scrollbar
 * overlay doesn't clip the last few pixels of each row.
 */
fn scrollbar_clearance() -> iced::Padding {
    iced::Padding {
        top: 0.0,
        right: style::SPACE_SM * 2f32,
        bottom: 0.0,
        left: 0.0,
    }
}

fn section_label(label: &str) -> Element<'_, Message> {
    text(label).size(11).color(style::TEXT_MUTED).into()
}

/**
 * A labeled row for the disk usage panel: the section name and its
 * used/total byte counts, with a small proportional bar underneath.
 */
fn section_usage_row(label: &str, used: u64, total: u64) -> Element<'_, Message> {
    let fraction = if total == 0 { 0.0 } else { used as f64 / total as f64 };
    column![
        row![
            text(label).size(13),
            Space::new().width(Length::Fill),
            text(format!("{} / {}", format_size(used), format_size(total)))
                .size(12)
                .color(style::TEXT_MUTED),
        ]
        .align_y(iced::Alignment::Center),
        usage_bar(fraction),
    ]
    .spacing(4.0)
    .into()
}

fn usage_bar<'a>(fraction: f64) -> Element<'a, Message> {
    let filled = ((fraction.clamp(0.0, 1.0)) * 1000.0).round() as u16;
    let filled = if fraction > 0.0 { filled.max(1) } else { 0 };
    let empty = 1000u16.saturating_sub(filled).max(1);
    row![
        container(Space::new())
            .width(Length::FillPortion(filled.max(1)))
            .height(Length::Fixed(6.0))
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(style::ACCENT)),
                border: iced::Border {
                    radius: style::RADIUS_PILL.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        container(Space::new())
            .width(Length::FillPortion(empty))
            .height(Length::Fixed(6.0))
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(style::SURFACE_ELEVATED)),
                border: iced::Border {
                    radius: style::RADIUS_PILL.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    ]
    .into()
}

fn file_type_label(file: &FileInstance) -> &'static str {
    if file.is_directory() {
        "directory"
    } else {
        "file"
    }
}

fn nav_row(label: &str, count: usize, selected: bool, message: Message) -> Element<'_, Message> {
    button(
        row![
            text(label.to_string()).size(14),
            Space::new().width(Length::Fill),
            text(count.to_string()).size(12).color(if selected {
                style::ACCENT
            } else {
                style::TEXT_MUTED
            }),
        ]
        .align_y(iced::Alignment::Center),
    )
    .style(style::nav_item(selected))
    .padding([style::SPACE_SM, style::SPACE_MD])
    .width(Length::Fill)
    .on_press(message)
    .into()
}

fn tag_chip(tag: &str) -> Element<'_, Message> {
    container(
        row![
            text(tag.to_string()).size(13),
            button(text("×").size(14))
                .style(style::chip_remove)
                .padding(0)
                .on_press(Message::RemoveTag(tag.to_string())),
        ]
        .spacing(style::SPACE_XS)
        .align_y(iced::Alignment::Center),
    )
    .style(style::chip)
    .padding([style::SPACE_XS, style::SPACE_SM])
    .into()
}
