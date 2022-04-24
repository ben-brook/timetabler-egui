use std::collections::{hash_map::Entry, HashMap};

use crate::timetabler;
use eframe::{egui, epi};

enum AppState {
    GeneralConfig,
    StudentConfig(bool),
    Submitted,
}

impl Default for AppState {
    fn default() -> Self {
        AppState::GeneralConfig
    }
}

const WEEK_DAYS: [&str; 5] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"];

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
#[derive(Default)]
pub struct TimetablerApp {
    max_groups_txt: String,
    max_groups: Option<u8>,
    daily_lesson_capacity_txt: String,
    daily_lesson_capacity: Option<u8>,
    state: AppState,
    subjects_by_student_id: HashMap<String, Vec<String>>,
    new_student_id_txt: String,
    new_student_subjects_txt: String,
    selected_student_id: String,
    result: Option<timetabler::TimetableResult>,
    // // this how you opt-out of serialization of a member
    // #[cfg_attr(feature = "persistence", serde(skip))]
    // value: f32
}

impl epi::App for TimetablerApp {
    fn name(&self) -> &str {
        "Timetabler"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _: &epi::Frame) {
        let Self {
            max_groups_txt,
            max_groups,
            daily_lesson_capacity_txt,
            daily_lesson_capacity,
            state,
            subjects_by_student_id,
            new_student_id_txt,
            new_student_subjects_txt,
            selected_student_id,
            result,
        } = self;

        *state = match &*state {
            AppState::GeneralConfig => {
                let mut new_state = AppState::GeneralConfig;

                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("General Configuration");

                    ui.horizontal(|ui| {
                        ui.label("Enter max groups: ");
                        ui.text_edit_singleline(max_groups_txt);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Enter daily lesson capacity: ");
                        ui.text_edit_singleline(daily_lesson_capacity_txt);
                    });

                    if ui.button("Next").clicked()
                        && match (max_groups_txt.parse(), daily_lesson_capacity_txt.parse()) {
                            (Ok(new_max_groups), Ok(new_daily_lesson_capacity)) => {
                                if new_max_groups > 0 && new_daily_lesson_capacity > 0 {
                                    *max_groups = Some(new_max_groups);
                                    *daily_lesson_capacity = Some(new_daily_lesson_capacity);
                                    true
                                } else {
                                    false
                                }
                            }
                            _ => false,
                        }
                    {
                        new_state = AppState::StudentConfig(false);
                    }
                });

                new_state
            }
            AppState::StudentConfig(is_creating) => {
                let mut new_state = AppState::StudentConfig(*is_creating);

                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Student Configuration");

                    egui::ComboBox::from_label("Select student")
                        .selected_text(selected_student_id.clone())
                        .show_ui(ui, |ui| {
                            for student_id in subjects_by_student_id.keys() {
                                ui.selectable_value(
                                    selected_student_id,
                                    student_id.clone(),
                                    student_id,
                                );
                            }
                        });

                    if ui.button("Add student").clicked() && !*is_creating {
                        new_state = AppState::StudentConfig(true);
                        new_student_id_txt.clear();
                        new_student_subjects_txt.clear();
                    }

                    if !selected_student_id.is_empty() {
                        ui.label(format!(
                            "Subjects: {}",
                            subjects_by_student_id[selected_student_id].join(",")
                        ));

                        if ui.button("Delete").clicked() {
                            subjects_by_student_id.remove(&selected_student_id.clone());
                            // We change the selected student ID since the current one doesn't exist anymore.
                            *selected_student_id = match subjects_by_student_id.keys().next() {
                                Some(id) => id.clone(),
                                None => String::new(),
                            }
                        }

                        // There is at least 1 student.
                        if ui.button("Submit").clicked() {
                            let mut student_infos = Vec::new();
                            for (student_id, subjects) in subjects_by_student_id.iter() {
                                let subjects =
                                    subjects.iter().map(|x| &x[..]).collect::<Vec<&str>>();
                                student_infos
                                    .push(timetabler::StudentInfo::new(student_id, subjects));
                            }
                            let info = timetabler::TimetableInfo {
                                // We can safely unwrap these two as for the app
                                // to be in this state, a value must have been
                                // provided to them already.
                                max_groups: max_groups.unwrap(),
                                daily_lesson_capacity: daily_lesson_capacity.unwrap(),
                                students: &student_infos,
                            };
                            *result = Some(timetabler::solve_timetable(&info));
                            new_state = AppState::Submitted;
                        }
                    }
                });

                if *is_creating && matches!(new_state, AppState::StudentConfig(_)) {
                    egui::Window::new("Create student").show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Enter ID: ");
                            ui.text_edit_singleline(new_student_id_txt);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Enter subject (comma separated): ");
                            ui.text_edit_singleline(new_student_subjects_txt);
                        });

                        ui.horizontal(|ui| {
                            let add_button = ui.button("Add");
                            let cancel_button = ui.button("Cancel");
                            if add_button.clicked() {
                                let entry =
                                    subjects_by_student_id.entry(new_student_id_txt.clone());
                                match entry {
                                    Entry::Vacant(vacant) => {
                                        if new_student_id_txt == "" {
                                            return;
                                        }
                                        let mut subjects = vec![];
                                        for subject in new_student_subjects_txt
                                            .split(',')
                                            .filter(|subject| !subject.is_empty())
                                        {
                                            if !subjects.contains(&subject.to_string()) {
                                                subjects.push(subject.to_string());
                                            }
                                        }
                                        if subjects.is_empty() {
                                            return;
                                        }
                                        vacant.insert(subjects);
                                        *selected_student_id = new_student_id_txt.clone();
                                        new_state = AppState::StudentConfig(false);
                                    }
                                    Entry::Occupied(_) => {}
                                };
                            } else if cancel_button.clicked() {
                                // We do else if so we don't handle both being
                                // clicked in the same frame, which would be
                                // bad.
                                new_state = AppState::StudentConfig(false);
                            }
                        });
                    });
                }

                new_state
            }
            AppState::Submitted => {
                if let Some(result) = &*result {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading("Result");

                        // We check that the timetable was solved and extract the values that
                        // the enum wraps.
                        if let timetabler::TimetableResult::Solved {
                            subjects,
                            slots_by_student_id,
                        } = result
                        {
                            ui.label("Global timetable");

                            // We get all the days into the days vector
                            let mut days: Vec<Vec<Vec<String>>> = Vec::new();
                            let mut current_day: Option<Vec<Vec<String>>> = None;
                            for (idx, slot) in subjects.iter().enumerate() {
                                if (idx) as u8 % daily_lesson_capacity.unwrap() == 0 {
                                    if let Some(day) = &current_day {
                                        days.push(day.clone());
                                    }
                                    current_day = Some(Vec::new());
                                }
                                if let Some(day) = &mut current_day {
                                    day.push(slot.clone());
                                }
                            }
                            // To get the last day in
                            if let Some(day) = &current_day {
                                days.push(day.clone());
                            }

                            // Set up the horizontal top section
                            ui.horizontal_top(|ui| {
                                for (idx, day) in days.iter().enumerate() {
                                    ui.vertical(|ui| {
                                        let week_day = WEEK_DAYS[idx];
                                        ui.label(week_day);
                                        // This grid is for the one specific week day
                                        egui::Grid::new(week_day).striped(true).show(ui, |ui| {
                                            for (slot, slot_subjects) in day.iter().enumerate() {
                                                ui.label(format!("Slot {}", slot + 1));
                                                ui.label(slot_subjects.join(", "));
                                                ui.end_row();
                                            }
                                        });
                                    });
                                }
                            });

                            egui::ComboBox::from_label("Select student")
                                .selected_text(selected_student_id.clone())
                                .show_ui(ui, |ui| {
                                    // For each key of the subjects_by_student_id hash map we
                                    // create a selectable value for it
                                    for student_id in subjects_by_student_id.keys() {
                                        ui.selectable_value(
                                            selected_student_id,
                                            student_id.clone(),
                                            student_id,
                                        );
                                    }
                                });

                            if !selected_student_id.is_empty() {
                                let slots = &slots_by_student_id[&selected_student_id.clone()];

                                let mut days: Vec<Vec<Option<String>>> = Vec::new();
                                let mut current_day: Option<Vec<Option<String>>> = None;
                                for (idx, subject) in slots.iter().enumerate() {
                                    if (idx) as u8 % daily_lesson_capacity.unwrap() == 0 {
                                        if let Some(day) = &current_day {
                                            days.push(day.clone());
                                        }
                                        current_day = Some(Vec::new());
                                    }
                                    if let Some(day) = &mut current_day {
                                        day.push(subject.as_ref().map(|subject| subject.0.clone()));
                                    }
                                }
                                // To get the last day in
                                if let Some(day) = &current_day {
                                    days.push(day.clone());
                                }

                                ui.horizontal_top(|ui| {
                                    for (idx, day) in days.iter().enumerate() {
                                        ui.vertical(|ui| {
                                            let week_day = WEEK_DAYS[idx];
                                            ui.label(week_day);
                                            // We need to create a new grid for each day as we
                                            // cannot approach this in a 3D manner
                                            egui::Grid::new(week_day).striped(true).show(
                                                ui,
                                                |ui| {
                                                    for (slot, slot_subject) in
                                                        day.iter().enumerate()
                                                    {
                                                        ui.label(format!("Slot {}", slot + 1));
                                                        ui.label(match slot_subject {
                                                            Some(subject) => subject,
                                                            None => "",
                                                        });
                                                        ui.end_row();
                                                    }
                                                },
                                            );
                                        });
                                    }
                                });
                            }
                        } else {
                            ui.label("Unable to solve. Try adjusting variables!");
                        }
                    });
                }

                AppState::Submitted
            }
        };
    }
}
