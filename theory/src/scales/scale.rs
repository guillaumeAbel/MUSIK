use super::notes::ENote;
use super::super::chords::chord;

#[derive(Debug, PartialEq)]
pub struct Scale {
    notes: Vec<ENote>
}

pub enum EScale {
    Major,
    MinorN
}

fn get_scale_intervals(scale: EScale) -> Vec<u8> {
    match scale {
        EScale::Major => vec![0, 2, 4, 5, 7, 9, 11],
        EScale::MinorN => vec![0, 2, 3, 5, 7, 8, 10]
    }
}

impl PartialEq<&[ENote]> for Scale {
    #[inline]
    fn eq(&self, cmp: &&[ENote]) -> bool {
        self.notes[..] == **cmp
    }
}

impl Scale {
    pub fn init_chord(&self, degree: usize, notes_nb: usize) -> chord::Chord {
        chord::init_chord(self, degree, notes_nb)
    }

    pub fn get(&self, i: usize) -> ENote {
        self.notes[i]
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }
}

pub fn init_scale(root: ENote, scale: EScale) -> Scale {
    let intervals = get_scale_intervals(scale);
    let notes: Vec<ENote> = 
        intervals
            .iter()
            .map(|interval| root + *interval)
            .collect();

    Scale { notes }
}
