use super::super::scales::{scale::Scale, notes::ENote};

#[derive(Debug, PartialEq)]
pub struct Chord {
    notes: Vec<ENote>
}

const MINOR_THIRD: u8 = 3;
const MAJOR_THIRD: u8 = 4;

impl Chord {
    #[cfg(test)]
    pub fn from_notes(notes: &[ENote]) -> Self {
        Chord {notes: notes.to_vec() }
    }
}

impl PartialEq<&[ENote]> for Chord {
    fn eq(&self, cmp: &&[ENote]) -> bool {
        self.notes[..] == **cmp
    }
}

fn check_chord_params(degree: usize, notes_nb: usize, scale_len: usize) {
    if !(1..=7).contains(&degree) {
        panic!("Out of range degree in get_chord()");
    } else if !(3..=5).contains(&notes_nb) {
        panic!("Out of range notes number in get_chord()");
    } else if !(3..=12).contains(&scale_len) {
        panic!("Unknown scale in get_chord()");
    }
}

fn next_interval(scale_index: &mut usize, scale: &Scale, scale_len: usize, note: ENote) -> ENote {
    *scale_index = (*scale_index + 1) % scale_len;

    let scale_note = scale.get(*scale_index);
    let minor = note + MINOR_THIRD;
    let major = note + MAJOR_THIRD;

    if scale_note == minor {
        return minor;
    } else if scale_note == major {
        return major;
    }
    next_interval(scale_index, scale, scale_len, note)
}

pub fn init_chord(scale: &Scale, degree: usize, notes_nb: usize) -> Chord {
    let scale_len = scale.len();
    check_chord_params(degree, notes_nb, scale_len);

    let mut notes = Vec::with_capacity(notes_nb);
    let mut scale_index = degree - 1;
    let mut note = scale.get(scale_index);

    notes.push(note);
    for _ in 1..notes_nb {
        note = next_interval(&mut scale_index, scale, scale_len, note);
        notes.push(note);
    }
    Chord { notes }
}
