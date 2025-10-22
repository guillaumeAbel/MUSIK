#[cfg(test)]
mod tests {
    use super::super::super::scales::{scale, notes::ENote::{self, *}};
    use scale::{init_scale, EScale, Scale};

    const NOTES_NB: usize = 4;
    const DEGREE_NB: usize = 7;

    const C_MAJOR_CHORDS: [[ENote; NOTES_NB]; DEGREE_NB] = [
        [C, E, G, B], [D, F, A, C], [E, G, B, D], [F, A, C, E], [G, B, D, F],
            [A, C, E, G], [B, D, F, A],
    ];

    const C_MINOR_CHORDS: [[ENote; NOTES_NB]; DEGREE_NB] = [
        [C, Ds, G, As], [D, F, Gs, C], [Ds, G, As, D], [F, Gs, C, Ds], [G, As, D, F],
        [Gs, C, Ds, G], [As, D, F, Gs],
    ];

    #[inline]
    fn test_chord(scale: &Scale, degree: usize, expected: &[ENote], notes_nb: usize) {
        assert_eq!(scale.init_chord(degree, notes_nb), expected);
    }

    fn test_scale_chords<const N: usize>(root: ENote, scale_type: EScale, expected: &[[ENote; N]]) {
        let scale = init_scale(root, scale_type);
        for (i, chord) in expected.iter().enumerate() {
            test_chord(&scale, i + 1, chord, N);
        }
    }

    #[test]
    fn c_major_chords() {
        test_scale_chords::<NOTES_NB>(C, EScale::Major, &C_MAJOR_CHORDS);
    }

    #[test]
    fn c_minor_chords() {
        test_scale_chords::<NOTES_NB>(C, EScale::MinorN, &C_MINOR_CHORDS);
    }
}
