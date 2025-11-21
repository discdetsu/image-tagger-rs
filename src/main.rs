use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use csv::StringRecord;
use iced::widget::text::Style as TextStyle;
use iced::widget::{self, button, column, container, pick_list, row, slider, text};
use iced::{
    application, event, keyboard, Alignment, Background, Border, Color, Element, Event, Length,
    Shadow, Size, Subscription, Task, Theme, Vector,
};
use serde::{Deserialize, Serialize};

const STATE_FILE_NAME: &str = ".tagger_state.json";
const APP_TITLE: &str = "Image Tagger";
const WINDOW_SIZE: (f32, f32) = (1400.0, 900.0);
const STATUS_BG_INFO: Color = color_from_rgb8(0xEE, 0xE8, 0xFF);
const STATUS_BG_SUCCESS: Color = color_from_rgb8(0xE2, 0xF7, 0xD3);
const STATUS_BG_ERROR: Color = color_from_rgb8(0xFF, 0xDF, 0xDC);
const APP_BACKGROUND: Color = color_from_rgb8(0x0D, 0x13, 0x24);
const SURFACE_PRIMARY: Color = color_from_rgb8(0x1C, 0x23, 0x38);
const SURFACE_SECONDARY: Color = color_from_rgb8(0x24, 0x2E, 0x45);
const ACCENT_PRIMARY: Color = color_from_rgb8(0x64, 0xA9, 0xFF);
const SUCCESS_ACCENT: Color = color_from_rgb8(0x34, 0xD3, 0x89);
const WARNING_ACCENT: Color = color_from_rgb8(0xFF, 0xC2, 0x6A);
const DANGER_ACCENT: Color = color_from_rgb8(0xFF, 0x7D, 0x7D);
const TEXT_PRIMARY: Color = color_from_rgb8(0xF4, 0xF5, 0xF7);
const TEXT_MUTED: Color = color_from_rgb8(0xA0, 0xA8, 0xC0);

const fn color_from_rgb8(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

fn main() -> iced::Result {
    let config = AppConfig::discover().unwrap_or_else(|err| {
        eprintln!("{APP_TITLE} unable to start: {err:#}");
        std::process::exit(1);
    });

    let boot_cfg = config.clone();

    application(
        |state: &TaggerState| state.title(),
        TaggerState::update,
        TaggerState::view,
    )
    .theme(|state| state.theme())
    .subscription(|_| TaggerState::subscription())
    .window_size(Size::new(WINDOW_SIZE.0, WINDOW_SIZE.1))
    .centered()
    .run_with(move || TaggerState::boot(boot_cfg.clone()))
}

#[derive(Debug, Clone)]
struct AppConfig {
    dataset_root: PathBuf,
    finding_name: String,
    state_file: PathBuf,
}

impl AppConfig {
    fn discover() -> Result<Self> {
        if let Ok(manual) = env::var("DATASET_ROOT") {
            let manual_path = PathBuf::from(manual);
            return Self::from_root(manual_path);
        }

        if let Some(arg_root) = env::args().skip(1).next() {
            if !arg_root.trim().is_empty() {
                return Self::from_root(PathBuf::from(arg_root));
            }
        }

        let mut current = env::current_dir().context("Read current directory")?;
        for _ in 0..6 {
            if looks_like_dataset_dir(&current)? {
                return Self::from_root(current);
            }

            if !current.pop() {
                break;
            }
        }

        Err(anyhow!(
            "Unable to locate dataset root. Pass it as the first CLI argument or set DATASET_ROOT."
        ))
    }

    fn from_root(root: PathBuf) -> Result<Self> {
        let canonical = root
            .canonicalize()
            .with_context(|| format!("Dataset root {} is invalid", root.display()))?;

        if !looks_like_dataset_dir(&canonical)? {
            return Err(anyhow!(
                "{path} does not look like a dataset folder",
                path = canonical.display()
            ));
        }

        let finding_name = canonical
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Finding")
            .to_string();

        let state_file = canonical.join(STATE_FILE_NAME);

        Ok(Self {
            dataset_root: canonical,
            finding_name,
            state_file,
        })
    }
}

#[derive(Debug)]
struct TaggerState {
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
    fn boot(config: AppConfig) -> (Self, Task<Message>) {
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

    fn try_boot(config: AppConfig) -> Result<Self> {
        let specs = collect_dataset_specs(&config.dataset_root)?;
        if specs.is_empty() {
            return Err(anyhow!(
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

    fn title(&self) -> String {
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

    fn theme(&self) -> Theme {
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

    fn update(&mut self, message: Message) -> Task<Message> {
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
        }
        Task::none()
    }

    fn subscription() -> Subscription<Message> {
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

    fn view(&self) -> Element<'_, Message> {
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
            .spacing(24)
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
                    .style(text_color(Color::from_rgb8(0x30, 0x30, 0x30))),
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
                    text(format!("Accession: {}", case.accession))
                        .size(18)
                        .style(text_color(TEXT_PRIMARY)),
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
            text(format!("{:.1}x", self.contrast))
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
                .padding([10, 20])
                .on_press(Message::Previous)
                .style(button::secondary),
            button("Next")
                .padding([10, 20])
                .on_press(Message::Next)
                .style(button::secondary),
        ]
        .spacing(12)
        .width(Length::Fill);

        let save_row = row![
            button("Save")
                .padding([10, 20])
                .on_press(Message::Save)
                .style(button::primary),
            button("Save & Exit")
                .padding([10, 20])
                .on_press(Message::SaveAndExit)
                .style(button::primary),
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
                    self.current_image = dataset_image_handle(
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
enum BannerKind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone)]
struct Banner {
    text: String,
    kind: BannerKind,
}

#[derive(Debug)]
struct DatasetState {
    spec: DatasetSpec,
    rows: Vec<CaseRecord>,
    current_index: usize,
    dirty: bool,
}

impl DatasetState {
    fn load(spec: &DatasetSpec, finding_column: &str) -> Result<Self> {
        let mut reader = csv::Reader::from_path(&spec.csv_path)
            .with_context(|| format!("Read CSV {}", spec.csv_path.display()))?;

        let headers = reader.headers()?.clone();
        let png_idx = column_index(&headers, "png_file", &spec.csv_path)?;
        let acc_idx = column_index(&headers, "acc", &spec.csv_path)?;
        let finding_idx = column_index(&headers, finding_column, &spec.csv_path)?;
        let confirm_idx = column_index(&headers, "confirm", &spec.csv_path)?;

        let mut rows = Vec::new();
        for record in reader.records() {
            let record = record?;
            rows.push(CaseRecord::from_record(
                &record,
                png_idx,
                acc_idx,
                finding_idx,
                confirm_idx,
            ));
        }

        Ok(Self {
            spec: spec.clone(),
            rows,
            current_index: 0,
            dirty: false,
        })
    }

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    fn current_case(&self) -> Option<&CaseRecord> {
        self.rows.get(self.current_index)
    }

    fn current_case_mut(&mut self) -> Option<&mut CaseRecord> {
        self.rows.get_mut(self.current_index)
    }

    fn resume_index(&self) -> usize {
        self.rows
            .iter()
            .position(|case| !case.confirm)
            .unwrap_or_else(|| self.rows.len().saturating_sub(1))
    }

    fn csv_file_name(&self) -> String {
        self.spec
            .csv_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.spec.name)
            .to_string()
    }

    fn image_path(&self, file: &str) -> PathBuf {
        self.spec.image_dir.join(file)
    }

    fn save(&mut self, finding_column: &str) -> Result<()> {
        let mut writer = csv::Writer::from_path(&self.spec.csv_path)
            .with_context(|| format!("Write CSV {}", self.spec.csv_path.display()))?;

        writer.write_record(["png_file", "acc", finding_column, "confirm"])?;
        for row in &mut self.rows {
            writer.write_record(row.to_record())?;
            row.dirty = false;
        }
        writer.flush()?;
        self.dirty = false;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct DatasetSpec {
    name: String,
    csv_path: PathBuf,
    image_dir: PathBuf,
}

impl DatasetSpec {
    fn new(name: String, root: &Path) -> Self {
        Self {
            csv_path: root.join(format!("{name}.csv")),
            image_dir: root.join(&name),
            name,
        }
    }
}

#[derive(Debug, Clone)]
struct CaseRecord {
    png_file: String,
    accession: String,
    finding: bool,
    confirm: bool,
    dirty: bool,
}

impl CaseRecord {
    fn from_record(
        record: &StringRecord,
        png_idx: usize,
        acc_idx: usize,
        finding_idx: usize,
        confirm_idx: usize,
    ) -> Self {
        Self {
            png_file: record.get(png_idx).unwrap_or_default().to_string(),
            accession: record.get(acc_idx).unwrap_or_default().to_string(),
            finding: parse_bool(record.get(finding_idx)),
            confirm: parse_bool(record.get(confirm_idx)),
            dirty: false,
        }
    }

    fn to_record(&self) -> [String; 4] {
        [
            self.png_file.clone(),
            self.accession.clone(),
            bool_to_string(self.finding),
            bool_to_string(self.confirm),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedState {
    selected_dataset: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    legacy_positions: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
enum Message {
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
}

fn collect_dataset_specs(root: &Path) -> Result<Vec<DatasetSpec>> {
    let mut stems = HashSet::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("csv") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    stems.insert(stem.to_string());
                }
            }
        }
    }

    let mut specs = Vec::new();
    for stem in stems {
        let dir_path = root.join(&stem);
        if dir_path.is_dir() {
            specs.push(DatasetSpec::new(stem, root));
        }
    }

    specs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(specs)
}

fn looks_like_dataset_dir(dir: &Path) -> Result<bool> {
    if !dir.is_dir() {
        return Ok(false);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("csv") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if dir.join(stem).is_dir() {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

fn column_index(headers: &StringRecord, name: &str, csv_path: &Path) -> Result<usize> {
    headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            anyhow!(
                "Column '{name}' not found in {file}",
                file = csv_path.display()
            )
        })
}

fn parse_bool(value: Option<&str>) -> bool {
    matches!(
        value.map(|v| v.trim()),
        Some("1") | Some("true") | Some("True") | Some("TRUE")
    )
}

fn bool_to_string(value: bool) -> String {
    if value {
        "1".to_string()
    } else {
        "0".to_string()
    }
}

fn dataset_image_handle(
    dataset: &DatasetState,
    file: &str,
    brightness: f32,
    contrast: f32,
    cache: &mut HashMap<PathBuf, DecodedImage>,
) -> Option<widget::image::Handle> {
    let path = dataset.image_path(file);
    let decoded = if let Some(existing) = cache.get(&path) {
        existing.clone()
    } else {
        let loaded = load_decoded_image(&path).ok()?;
        cache.insert(path.clone(), loaded.clone());
        loaded
    };

    Some(apply_luma_adjust(&decoded, brightness, contrast))
}

fn load_state(path: &Path) -> PersistedState {
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(parsed) = serde_json::from_str(&data) {
            return parsed;
        }
    }
    PersistedState::default()
}

fn save_state_file(path: &Path, state: &PersistedState) -> Result<()> {
    let json = serde_json::to_string_pretty(state)?;
    fs::write(path, json)?;
    Ok(())
}

#[derive(Debug, Clone)]
struct DecodedImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn load_decoded_image(path: &Path) -> Result<DecodedImage> {
    let img = image::open(path).with_context(|| format!("Open image {}", path.display()))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(DecodedImage {
        width,
        height,
        rgba: rgba.into_raw(),
    })
}

fn apply_luma_adjust(
    image: &DecodedImage,
    brightness: f32,
    contrast: f32,
) -> widget::image::Handle {
    let mut adjusted = image.rgba.clone();
    let b = brightness.max(0.0);
    let c = contrast.max(0.0);
    for chunk in adjusted.chunks_mut(4) {
        let apply = |v: u8| {
            let mut val = v as f32 * b;
            val = ((val - 128.0) * c) + 128.0;
            val.clamp(0.0, 255.0) as u8
        };
        chunk[0] = apply(chunk[0]);
        chunk[1] = apply(chunk[1]);
        chunk[2] = apply(chunk[2]);
    }

    widget::image::Handle::from_rgba(image.width, image.height, adjusted)
}

fn text_color(color: Color) -> impl Fn(&Theme) -> TextStyle {
    move |_| TextStyle {
        color: Some(color),
        ..TextStyle::default()
    }
}

fn background_panel_style(_theme: &Theme) -> widget::container::Style {
    widget::container::Style {
        background: Some(Background::Color(APP_BACKGROUND)),
        ..widget::container::Style::default()
    }
}

fn image_frame_style(_theme: &Theme) -> widget::container::Style {
    widget::container::Style {
        background: Some(Background::Color(SURFACE_SECONDARY)),
        border: Border::default().rounded(28),
        shadow: Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.35,
            },
            offset: Vector::new(0.0, 16.0),
            blur_radius: 32.0,
        },
        ..widget::container::Style::default()
    }
}

fn surface_card_style(_theme: &Theme) -> widget::container::Style {
    widget::container::Style {
        background: Some(Background::Color(SURFACE_PRIMARY)),
        border: Border::default().rounded(20),
        shadow: Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.25,
            },
            offset: Vector::new(0.0, 14.0),
            blur_radius: 28.0,
        },
        ..widget::container::Style::default()
    }
}

fn banner_style(color: Color) -> widget::container::Style {
    widget::container::Style {
        background: Some(Background::Color(color)),
        border: Border::default().rounded(12),
        ..widget::container::Style::default()
    }
}

fn badge_style(color: Color) -> widget::container::Style {
    widget::container::Style {
        background: Some(Background::Color(color)),
        border: Border::default().rounded(30),
        ..widget::container::Style::default()
    }
}

fn darken(color: Color, amount: f32) -> Color {
    let factor = 1.0 - amount;
    Color {
        r: (color.r * factor).clamp(0.0, 1.0),
        g: (color.g * factor).clamp(0.0, 1.0),
        b: (color.b * factor).clamp(0.0, 1.0),
        a: color.a,
    }
}

fn active_success_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let mut style = button::success(theme, status);
    if active {
        style.background = style
            .background
            .map(|bg| match bg {
                Background::Color(c) => Background::Color(darken(c, 0.5)),
                other => other,
            });
    }
    style
}

fn active_danger_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let mut style = button::danger(theme, status);
    if active {
        style.background = style
            .background
            .map(|bg| match bg {
                Background::Color(c) => Background::Color(darken(c, 0.5)),
                other => other,
            });
    }
    style
}
