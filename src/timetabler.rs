use std::collections::HashMap;

pub struct StudentInfo<'a> {
    id: &'a str,
    subjects: Vec<&'a str>,
}

impl<'a> StudentInfo<'a> {
    pub fn new(id: &'a str, subjects: Vec<&'a str>) -> StudentInfo<'a> {
        StudentInfo { id, subjects }
    }
}

pub struct TimetableInfo<'a> {
    pub max_groups: u8,
    pub students: &'a Vec<StudentInfo<'a>>,
    pub daily_lesson_capacity: u8,
}

#[derive(Debug)] // Allow the struct to be printed for debugging.
pub struct Student {
    slots: Vec<Option<(String, usize)>>,
    id: String,
}

#[derive(Debug)] // Allow the struct to be printed for debugging.
pub enum TimetableResult {
    Solved {
        subjects: Vec<Vec<String>>,
        slots_by_student_id: HashMap<String, Vec<Option<(String, usize)>>>,
    },
    Unsolved,
}

#[derive(Default, Debug)]
struct Group {
    slot: usize,
    student_idxs: Vec<usize>,
}

fn attendance(candidate: (&str, usize), groups_by_subject: &HashMap<&str, Vec<Group>>) -> usize {
    groups_by_subject[candidate.0]
        .get(candidate.1)
        .map(|group| group.student_idxs.len())
        .unwrap_or_default()
}

fn sort_by_ascending_attendance(
    candidates: &mut Vec<(&str, usize)>,
    groups_by_subject: &HashMap<&str, Vec<Group>>,
    start: i32,
    end: i32,
) {
    if start >= end {
        return;
    }

    let pivot = attendance(candidates[start as usize], groups_by_subject);
    let mut low_mark = start + 1;
    let mut high_mark = end;

    loop {
        while low_mark <= high_mark
            && attendance(candidates[low_mark as usize], groups_by_subject) <= pivot
        {
            low_mark += 1;
        }
        while low_mark <= high_mark
            && attendance(candidates[high_mark as usize], groups_by_subject) >= pivot
        {
            high_mark -= 1;
        }
        if low_mark < high_mark {
            candidates.swap(low_mark as usize, high_mark as usize);
        } else {
            break;
        }
    }

    candidates.swap(start as usize, high_mark as usize);
    sort_by_ascending_attendance(candidates, groups_by_subject, start, high_mark - 1);
    sort_by_ascending_attendance(candidates, groups_by_subject, high_mark + 1, end);
}

fn try_assign_group_lazily<'a>(
    groups_by_subject: &HashMap<&str, Vec<Group>>,
    personal_slots: &mut Vec<Option<(&'a str, usize)>>,
    subject: &'a str,
) -> bool {
    // We iterate over each group of the subject that currently exists.
    // For each, we check if it can be used.
    for (group_idx, group) in groups_by_subject
        .get(subject)
        .unwrap_or(&Vec::new())
        .iter()
        .enumerate()
    {
        if personal_slots[group.slot].is_some() {
            // The slot is taken by another subject.
            continue;
        }
        personal_slots[group.slot] = Some((subject, group_idx));
        return true;
    }

    false
}

fn handle_subjects<'a>(
    groups_by_subject: &mut HashMap<&'a str, Vec<Group>>,
    personal_slots: &mut Vec<Option<(&'a str, usize)>>,
    subjects: &Vec<&'a str>,
    timetable_info: &TimetableInfo,
    total_slots: u8,
    students: &mut Vec<Student>,
) -> bool {
    for &subject in subjects {
        if try_assign_group_lazily(groups_by_subject, personal_slots, subject) {
            // We don't need to continue as we could find a suitable group.
            continue;
        }

        // We can just unwrap this as we validate that the student always
        // has enough personal slots to cover all their subjects.
        let next_free_slot = personal_slots.iter().position(|x| x.is_none()).unwrap();

        if groups_by_subject.get(subject).unwrap_or(&Vec::new()).len()
            == timetable_info.max_groups.into()
        {
            // Groups are at capacity. One of this student's subject groups,
            // including the potential current one, needs to be moved to the
            // next free slot. In order of ascending attendee count, check
            // each for each next free slot available.

            // Candidates include all groups of the current subject.
            let mut candidates: Vec<(&str, usize)> =
                personal_slots.clone().into_iter().flatten().collect();
            for i in 0..groups_by_subject[subject].len() {
                candidates.push((subject, i));
            }

            let end = candidates.len() as i32 - 1;
            sort_by_ascending_attendance(&mut candidates, &groups_by_subject, 0, end);

            // We choose a subject group to move.
            let mut next_free_slot = next_free_slot;
            let mut personal_slots_iter = personal_slots.iter().enumerate();
            let chosen = 'outer: loop {
                for &(candidate_subject, candidate_group_idx) in &candidates {
                    let mut is_candidate_ok = true;
                    if let Some(group) =
                        groups_by_subject[candidate_subject].get(candidate_group_idx)
                    {
                        for &other_student_idx in group.student_idxs.iter() {
                            let other_student = &mut students[other_student_idx];
                            if other_student.slots[next_free_slot].is_some() {
                                is_candidate_ok = false;
                                break;
                            }
                        }
                    }

                    if is_candidate_ok {
                        break 'outer Some((candidate_subject, candidate_group_idx));
                    }
                }

                println!("{next_free_slot}");
                if next_free_slot == (total_slots - 1).into() {
                    break None;
                }
                next_free_slot = personal_slots_iter
                    .position(|(pos, x)| pos > next_free_slot && x.is_none())
                    // We can just unwrap this as we validate that the
                    // student always has enough personal slots to cover all
                    // their subjects.
                    .unwrap();
            };

            if let Some((chosen_subject, chosen_group_idx)) = chosen {
                let chosen_group_slot;

                if let Some(chosen_group) = groups_by_subject
                    // We are `.get_mut(...).unwrap()`ing since currently we
                    // cannot index mutably into HashMaps in Rust.
                    .get_mut(chosen_subject)
                    .unwrap()
                    .get_mut(chosen_group_idx)
                {
                    for &other_student_idx in &chosen_group.student_idxs {
                        // This doesn't include the current student.
                        let other_student = &mut students[other_student_idx];
                        other_student.slots[chosen_group.slot] = None;
                        other_student.slots[next_free_slot] =
                            Some((chosen_subject.to_string(), chosen_group_idx));
                    }

                    chosen_group_slot = chosen_group.slot;
                    chosen_group.slot = next_free_slot;
                } else {
                    chosen_group_slot = personal_slots
                        .iter()
                        .position(|x| x.is_some() && x.unwrap().0 == chosen_subject)
                        .unwrap();
                }

                if chosen_subject == subject {
                    personal_slots[next_free_slot] = Some((subject, chosen_group_idx));
                } else {
                    personal_slots[next_free_slot] = personal_slots[chosen_group_slot];
                    // We finally add the subject to the personal slot.
                    personal_slots[chosen_group_slot] = Some((subject, chosen_group_idx));
                }
            } else {
                return true;
            }
        } else {
            // Groups aren't at capacity, so we can create a new group at
            // the earliest possible position in the student's personal
            // slots.

            personal_slots[next_free_slot] = Some((
                subject,
                groups_by_subject.entry(subject).or_insert(Vec::new()).len(),
            ));
        }
    }

    false
}

fn make_global<'a>(
    groups_by_subject: &mut HashMap<&'a str, Vec<Group>>,
    personal_slots: &mut Vec<Option<(&'a str, usize)>>,
    student_idx: usize,
) {
    for (slot, (subject, group_idx)) in personal_slots
        .iter()
        .enumerate()
        .flat_map(|(i, c)| c.map(|c| (i, c)))
    {
        let groups = groups_by_subject.entry(subject).or_insert(Vec::new());
        if let Some(group) = groups.get_mut(group_idx) {
            // There will never be more than one group per subject per
            // student, so we can just push.
            group.student_idxs.push(student_idx);
        } else {
            groups.push(Group {
                slot,
                student_idxs: vec![student_idx],
            });
        }
    }
}

pub fn solve_timetable(timetable_info: &TimetableInfo<'_>) -> TimetableResult {
    let mut students: Vec<Student> = Vec::new();

    // There are 5 days in the timetable week.
    let total_slots = timetable_info.daily_lesson_capacity * 5;
    let mut groups_by_subject: HashMap<&str, Vec<Group>> = HashMap::new();
    for (student_idx, student_info) in timetable_info.students.iter().enumerate() {
        // We map slots to possible subjects here.
        let mut personal_slots = vec![None; total_slots.into()];
        let subjects = &student_info.subjects;
        if handle_subjects(
            &mut groups_by_subject,
            &mut personal_slots,
            subjects,
            timetable_info,
            total_slots,
            &mut students,
        ) {
            return TimetableResult::Unsolved;
        }

        // We add the groups we decided upon to the global vector.
        make_global(&mut groups_by_subject, &mut personal_slots, student_idx);

        // We just turn Subject &strs into Strings so that the Student instance
        // can own them.
        let mut returned_personal_slots = Vec::new();
        for slot in personal_slots {
            returned_personal_slots
                .push(slot.map(|(subject, group_idx)| (subject.to_string(), group_idx)));
        }

        // We register the student to keep track of for later.
        students.push(Student {
            slots: returned_personal_slots,
            id: student_info.id.to_string(),
        });
    }

    // We invert groups_by_subject to help get subjects_by_slot.
    let mut subjects = vec![Vec::new(); total_slots.into()];
    for (subject, groups) in groups_by_subject {
        for group in groups {
            // It's guaranteed that this will never cause duplicate subjects, so
            // we don't need to check.
            subjects[group.slot].push(subject.to_string());
        }
    }

    let mut slots_by_student_id = HashMap::new();
    for student in students {
        slots_by_student_id.insert(student.id, student.slots);
    }

    TimetableResult::Solved {
        subjects,
        slots_by_student_id,
    }
}
