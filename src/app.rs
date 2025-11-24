use std::collections::HashMap;
use std::path::PathBuf;

use iced::widget::{self, button, column, container, pick_list, row, slider, text};
use iced::{
    event, keyboard, Alignment, Color, Element, Event, Length, Subscription,
    Task, Theme,
};

use crate::config::AppConfig;
use crate::dataset::{collect_dataset_specs, DatasetState};
use crate::image::{load_image_with_cache, DecodedImage};
use crate::state::{load_state, save_state_file, PersistedState};
use crate::ui::*;

pub const APP_TITLE: &str = "Image Tagger";
pub const WINDOW_SIZE: (f32, f32) = (1400.0, 900.0);

#[derive(Debug)]
pub struct TaggerState {
    config: AppConfig,
    datasets: Vec<DatasetState>,
    dataset_lookup: HashMap<String, usize>,
    dataset_names: Vec<String>,
    selected_dataset: Option<usize>,
    selection_label: Option<String>,
    current_image: Option<widget::image::Handle>,
    status: Option<Banner>,
    persisted: PersistedState,
    brightness: f32,
    contrast: f32,
    image_cache: HashMap<PathBuf, DecodedImage>,
}

impl TaggerState {
    pub fn boot(config: AppConfig) -> (Self, Task<Message>) {
        match Self::try_boot(config.clone()) {
            Ok(mut state) => {
                state.refresh_image();
                state.persist_state();
                (state, Task::none())
            }
            Err(err) => {
                let mut state = Self::empty(config);
                state.set_status(
                    format!("Failed to load datasets: {err:#}"),
                    BannerKind::Error,
                );
                (state, Task::none())
            }
        }
    }

    fn try_boot(config: AppConfig) -> anyhow::Result<Self> {
        let specs = collect_dataset_specs(&config.dataset_root)?;
        if specs.is_empty() {
            return Err(anyhow::anyhow!(
                "No dataset folders were found under {}",
                config.dataset_root.display()
            ));
        }

        let mut persisted = load_state(&config.state_file);
        let mut datasets = Vec::new();
        let mut dataset_lookup = HashMap::new();
        let mut dataset_names = Vec::new();

        for spec in specs {
            let mut dataset = DatasetState::load(&spec, &config.finding_name)?;
            dataset.current_index = dataset.resume_index();

            dataset_lookup.insert(spec.name.clone(), datasets.len());
            dataset_names.push(spec.name.clone());
            datasets.push(dataset);
        }

        let selected_dataset = persisted
            .selected_dataset
            .as_ref()
            .and_then(|name| dataset_lookup.get(name).copied())
            .or_else(|| if datasets.is_empty() { None } else { Some(0) });

        let selection_label = selected_dataset
            .and_then(|idx| datasets.get(idx))
            .map(|dataset| dataset.spec.name.clone());

        persisted.selected_dataset = selection_label.clone();

        Ok(Self {
            config,
            datasets,
            dataset_lookup,
            dataset_names,
            selected_dataset,
            selection_label,
            current_image: None,
            status: None,
            persisted,
            brightness: 1.0,
            contrast: 1.0,
            image_cache: HashMap::new(),
        })
    }

    fn empty(config: AppConfig) -> Self {
        Self {
            config,
            datasets: Vec::new(),
            dataset_lookup: HashMap::new(),
            dataset_names: Vec::new(),
            selected_dataset: None,
            selection_label: None,
            current_image: None,
            status: None,
            persisted: PersistedState::default(),
            brightness: 1.0,
            contrast: 1.0,
            image_cache: HashMap::new(),
        }
    }

    pub fn title(&self) -> String {
        if let Some(name) = self
            .selected_dataset
            .and_then(|idx| self.datasets.get(idx))
            .map(|dataset| dataset.spec.name.clone())
        {
            format!("{APP_TITLE} — {} ({})", name, self.config.finding_name)
        } else {
            format!("{APP_TITLE} — {}", self.config.finding_name)
        }
    }

    pub fn theme(&self) -> Theme {
        Theme::custom(
            "tagger".to_string(),
            iced::theme::Palette {
                background: APP_BACKGROUND,
                text: TEXT_PRIMARY,
                primary: ACCENT_PRIMARY,
                success: SUCCESS_ACCENT,
                danger: DANGER_ACCENT,
            },
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DatasetPicked(name) => {
                self.change_dataset(&name);
            }
            Message::Previous => {
                self.go_previous();
            }
            Message::Next => {
                self.go_next();
            }
            Message::MarkYes => {
                self.apply_label(true);
            }
            Message::MarkNo => {
                self.apply_label(false);
            }
            Message::Save => {
                self.save_selected_dataset(false);
            }
            Message::SaveAndExit => {
                if self.save_selected_dataset(true) {
                    return iced::exit();
                }
            }
            Message::EventOccurred(evt) => {
                self.handle_event(evt);
            }
            Message::DismissStatus => {
                self.status = None;
            }
            Message::BrightnessChanged(value) => {
                self.brightness = value;
                self.refresh_image();
            }
            Message::ContrastChanged(value) => {
                self.contrast = value;
                self.refresh_image();
            }
            Message::CopyAccession(accession) => {
                return iced::clipboard::write(accession.clone())
                    .map(move |()| Message::AccessionCopied(accession.clone()));
            }
            Message::AccessionCopied(accession) => {
                self.set_status(
                    format!("Copied accession {} to clipboard", accession),
                    BannerKind::Info,
                );
            }
        }
        Task::none()
    }

    pub fn subscription() -> Subscription<Message> {
        event::listen().map(Message::EventOccurred)
    }

    fn handle_event(&mut self, evt: Event) {
        if let Event::Keyboard(keyboard::Event::KeyPressed { key, text, .. }) = evt {
            match key.as_ref() {
                keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => self.go_previous(),
                keyboard::Key::Named(keyboard::key::Named::ArrowRight) => self.go_next(),
                _ => match text.as_deref() {
                    Some("1") => self.apply_label(true),
                    Some("2") => self.apply_label(false),
                    _ => {}
                },
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.datasets.is_empty() {
            return container(
                text("No datasets were found in the current folder.")
                    .size(28)
                    .style(text_color(Color::from_rgb8(0x44, 0x44, 0x44))),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into();
        }

        let image_section = container(self.image_panel())
            .width(Length::FillPortion(3))
            .height(Length::Fill);

        let control_section = container(self.control_panel()).width(Length::FillPortion(1));

        let main_row = row![image_section, control_section]
            .spacing(32) // Increased spacing
            .height(Length::Fill)
            .align_y(Alignment::Start);

        let content: Element<_> = if let Some(banner) = &self.status {
            let banner_color = match banner.kind {
                BannerKind::Info => STATUS_BG_INFO,
                BannerKind::Success => STATUS_BG_SUCCESS,
                BannerKind::Error => STATUS_BG_ERROR,
            };

            let banner_row_content = row![
                text(&banner.text)
                    .size(18)
                    .style(text_color(TEXT_PRIMARY)),
                button("Dismiss")
                    .padding([6, 14])
                    .on_press(Message::DismissStatus),
            ]
            .spacing(12)
            .align_y(Alignment::Center);

            let color = banner_color;
            column![
                container(banner_row_content)
                    .style(move |_| banner_style(color))
                    .padding(12)
                    .width(Length::Fill),
                main_row,
            ]
            .spacing(16)
            .into()
        } else {
            main_row.into()
        };

        container(content)
            .padding(24)
            .style(background_panel_style)
            .into()
    }

    fn image_panel(&self) -> Element<'_, Message> {
        let content: Element<_> = if let Some(handle) = &self.current_image {
            widget::image::viewer(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else if let Some(dataset) = self.current_dataset() {
            if dataset.is_empty() {
                text("Dataset is empty").size(24).into()
            } else if let Some(case) = dataset.current_case() {
                let missing = dataset.image_path(&case.png_file);
                text(format!("Image missing: {}", missing.display()))
                    .size(20)
                    .style(text_color(Color::from_rgb(0.8, 0.1, 0.1)))
                    .into()
            } else {
                text("Select a case to begin").size(24).into()
            }
        } else {
            text("Select a dataset to begin").size(24).into()
        };

        container(content)
            .padding(12)
            .style(image_frame_style)
            .height(Length::Fill)
            .into()
    }

    fn control_panel(&self) -> Element<'_, Message> {
        let dataset_pick = pick_list(
            self.dataset_names.clone(),
            self.selection_label.clone(),
            Message::DatasetPicked,
        )
        .width(Length::Fill)
        .padding(10);

        let selector_card = container(
            column![
                text("Dataset").size(16).style(text_color(TEXT_MUTED)),
                dataset_pick,
            ]
            .spacing(8),
        )
        .padding(16)
        .style(surface_card_style)
        .width(Length::Fill);

        let dataset_info = if let Some(dataset) = self.current_dataset() {
            if let Some(case) = dataset.current_case() {
                let (badge_text, badge_color, badge_text_color) = if case.confirm {
                    (
                        "Already tagged",
                        SUCCESS_ACCENT,
                        color_from_rgb8(0x05, 0x3B, 0x2E),
                    )
                } else {
                    (
                        "Needs tagging",
                        WARNING_ACCENT,
                        color_from_rgb8(0x5E, 0x3B, 0x00),
                    )
                };

                let badge_color_copy = badge_color;
                column![
                    text(&dataset.spec.name)
                        .size(20)
                        .style(text_color(TEXT_PRIMARY)),
                    text(format!(
                        "Case {} / {}",
                        dataset.current_index + 1,
                        dataset.len()
                    ))
                    .size(18)
                    .style(text_color(TEXT_MUTED)),
                    row![
                        text(format!("Accession: {}", case.accession))
                            .size(18)
                            .style(text_color(TEXT_PRIMARY)),
                        button(text("Copy").size(12))
                            .padding([4, 8])
                            .on_press(Message::CopyAccession(case.accession.clone()))
                            .style(secondary_button_style),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    text(format!(
                        "Tagged: {}",
                        if case.confirm { "Yes" } else { "No" }
                    ))
                    .size(17)
                    .style(text_color(TEXT_MUTED)),
                    container(
                        text(badge_text)
                            .size(14)
                            .style(text_color(badge_text_color)),
                    )
                    .padding([6, 12])
                    .style(move |_| badge_style(badge_color_copy)),
                ]
                .spacing(8)
            } else {
                column![text("Dataset has no rows")
                    .size(18)
                    .style(text_color(TEXT_MUTED))]
            }
        } else {
            column![text("Pick a dataset to start tagging")
                .size(18)
                .style(text_color(TEXT_MUTED))]
        };

        let info_card = container(dataset_info)
            .padding(20)
            .style(surface_card_style)
            .width(Length::Fill);

        let brightness_row = row![
            text("Brightness")
                .size(14)
                .style(text_color(TEXT_MUTED)),
            text(format!("{:.0}%", self.brightness * 100.0))
                .size(14)
                .style(text_color(TEXT_MUTED))
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let brightness_slider = slider(0.2..=2.0, self.brightness, Message::BrightnessChanged)
            .step(0.05)
            .width(Length::Fill);

        let contrast_row = row![
            text("Contrast")
                .size(14)
                .style(text_color(TEXT_MUTED)),
            text(format!("{:.2}x", self.contrast))
                .size(14)
                .style(text_color(TEXT_MUTED))
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let contrast_slider = slider(0.5..=2.0, self.contrast, Message::ContrastChanged)
            .step(0.05)
            .width(Length::Fill);

        let image_adjust_card = container(
            column![brightness_row, brightness_slider, contrast_row, contrast_slider]
                .spacing(10)
                .width(Length::Fill),
        )
        .padding(16)
        .style(surface_card_style)
        .width(Length::Fill);

        let (yes_active, no_active) = if let Some(case) = self
            .current_dataset()
            .and_then(|dataset| dataset.current_case())
        {
            (case.confirm && case.finding, case.confirm && !case.finding)
        } else {
            (false, false)
        };

        let yes_button = button("Yes")
            .padding([12, 28])
            .on_press(Message::MarkYes)
            .style(move |theme, status| active_success_style(theme, status, yes_active));

        let no_button = button("No")
            .padding([12, 28])
            .on_press(Message::MarkNo)
            .style(move |theme, status| active_danger_style(theme, status, no_active));

        let nav_row = row![
            button("Previous")
                .padding([12, 24])
                .on_press(Message::Previous)
                .style(secondary_button_style)
                .width(Length::Fill),
            button("Next")
                .padding([12, 24])
                .on_press(Message::Next)
                .style(secondary_button_style)
                .width(Length::Fill),
        ]
        .spacing(12)
        .width(Length::Fill);

        let save_row = row![
            button("Save")
                .padding([12, 24])
                .on_press(Message::Save)
                .style(primary_button_style)
                .width(Length::Fill),
            button("Save & Exit")
                .padding([12, 24])
                .on_press(Message::SaveAndExit)
                .style(primary_button_style)
                .width(Length::Fill),
        ]
        .spacing(12)
        .width(Length::Fill);

        let helper_text = text("Hotkeys:  ← Previous,  → Next,  1 = Yes,  2 = No")
            .size(14)
            .style(text_color(TEXT_MUTED));

        let actions_card = container(
            column![
                text("Tagging Actions")
                    .size(16)
                    .style(text_color(TEXT_MUTED)),
                row![yes_button, no_button]
                    .spacing(12)
                    .width(Length::Fill)
                    .align_y(Alignment::Center),
                nav_row,
                save_row,
                helper_text,
            ]
            .spacing(16),
        )
        .padding(20)
        .style(surface_card_style)
        .width(Length::Fill);

        column![selector_card, info_card, image_adjust_card, actions_card]
            .spacing(18)
            .into()
    }

    fn change_dataset(&mut self, name: &str) {
        if let Some(&idx) = self.dataset_lookup.get(name) {
            self.selected_dataset = Some(idx);
            self.selection_label = Some(name.to_string());
            self.refresh_image();
            self.persisted.selected_dataset = Some(name.to_string());
            self.persist_state();
        }
    }

    fn current_dataset(&self) -> Option<&DatasetState> {
        self.selected_dataset.and_then(|idx| self.datasets.get(idx))
    }

    fn current_dataset_mut(&mut self) -> Option<&mut DatasetState> {
        let idx = self.selected_dataset?;
        self.datasets.get_mut(idx)
    }

    fn go_previous(&mut self) {
        let moved = {
            let Some(dataset) = self.current_dataset_mut() else {
                return;
            };

            if dataset.current_index > 0 {
                dataset.current_index -= 1;
                true
            } else {
                false
            }
        };

        if moved {
            self.persist_current_position();
            self.refresh_image();
        }
    }

    fn go_next(&mut self) {
        let moved = {
            let Some(dataset) = self.current_dataset_mut() else {
                return;
            };

            if dataset.current_index + 1 < dataset.len() {
                dataset.current_index += 1;
                true
            } else {
                false
            }
        };

        if moved {
            self.persist_current_position();
            self.refresh_image();
        }
    }

    fn apply_label(&mut self, value: bool) {
        let Some(idx) = self.selected_dataset else {
            return;
        };

        let tagged_file = {
            let Some(dataset) = self.datasets.get_mut(idx) else {
                return;
            };

            let result = if let Some(case) = dataset.current_case_mut() {
                if case.finding != value || !case.confirm {
                    case.finding = value;
                    case.confirm = true;
                    case.dirty = true;
                    Some(case.png_file.clone())
                } else {
                    None
                }
            } else {
                None
            };

            if result.is_some() {
                dataset.dirty = true;
            }

            result
        };

        if let Some(file) = tagged_file {
            self.set_status(
                format!("Tagged {} as {}", file, if value { "Yes" } else { "No" }),
                BannerKind::Info,
            );
        }

        self.go_next();
    }

    fn save_selected_dataset(&mut self, exit: bool) -> bool {
        let Some(idx) = self.selected_dataset else {
            self.set_status("Select a dataset before saving", BannerKind::Error);
            return false;
        };

        let (dataset_name, dataset_len, save_result) = {
            let dataset = &mut self.datasets[idx];
            let name = dataset.csv_file_name();
            let len = dataset.len();
            let result = dataset.save(&self.config.finding_name);
            (name, len, result)
        };

        match save_result {
            Ok(_) => {
                if let Some(dataset) = self.datasets.get_mut(idx) {
                    dataset.current_index = dataset.resume_index();
                }
                self.refresh_image();
                self.set_status(
                    format!("Saved {} ({} cases)", dataset_name, dataset_len),
                    BannerKind::Success,
                );
                self.persist_current_position();
                exit
            }
            Err(err) => {
                self.set_status(
                    format!("Failed to save {}: {err:#}", dataset_name),
                    BannerKind::Error,
                );
                false
            }
        }
    }

    fn persist_current_position(&mut self) {
        if let Some(idx) = self.selected_dataset {
            if let Some(dataset) = self.datasets.get(idx) {
                self.persisted.selected_dataset = Some(dataset.spec.name.clone());
                self.persist_state();
            }
        }
    }

    fn persist_state(&self) {
        if let Err(err) = save_state_file(&self.config.state_file, &self.persisted) {
            eprintln!("Failed to persist state: {err:#}");
        }
    }

    fn refresh_image(&mut self) {
        if let Some(idx) = self.selected_dataset {
            if let Some(dataset) = self.datasets.get(idx) {
                if let Some(case) = dataset.current_case() {
                    self.current_image = load_image_with_cache(
                        dataset,
                        &case.png_file,
                        self.brightness,
                        self.contrast,
                        &mut self.image_cache,
                    );
                    return;
                }
            }
        }

        self.current_image = None;
    }

    fn set_status(&mut self, text: impl Into<String>, kind: BannerKind) {
        self.status = Some(Banner {
            text: text.into(),
            kind,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerKind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub struct Banner {
    pub text: String,
    pub kind: BannerKind,
}

#[derive(Debug, Clone)]
pub enum Message {
    DatasetPicked(String),
    Previous,
    Next,
    MarkYes,
    MarkNo,
    Save,
    SaveAndExit,
    EventOccurred(Event),
    DismissStatus,
    BrightnessChanged(f32),
    ContrastChanged(f32),
    CopyAccession(String),
    AccessionCopied(String),
}
