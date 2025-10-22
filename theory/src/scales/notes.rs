use std::ops::{Add, AddAssign};

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum ENote {
    C, Cs, D, Ds, E, F, Fs, G, Gs, A, As, B
}

impl ENote {
    const ALL: [ENote; 12] = [
        ENote::C, ENote::Cs, ENote::D, ENote::Ds, ENote::E, ENote::F,
        ENote::Fs, ENote::G, ENote::Gs, ENote::A, ENote::As, ENote::B
    ];

    #[inline]
    fn to_index(self) -> u8 {
        self as u8
    }

    #[inline]
    fn from_index(i: u8) -> ENote {
        if i > 11 {
            panic!("Note index out of range in from_index")
        }
        Self::ALL[i as usize]
    }
}

impl Add<u8> for ENote {
    type Output = ENote;

    #[inline]
    fn add(self, x: u8) -> Self::Output {
        ENote::from_index((self.to_index() + x) % 12)
    }
}

impl AddAssign<u8> for ENote {
    #[inline]
    fn add_assign(&mut self, x: u8) {
        *self = *self + x;
    }
}
