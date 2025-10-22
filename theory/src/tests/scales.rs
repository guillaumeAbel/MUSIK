#[cfg(test)]
mod tests {
    use super::super::super::scales::{scale, notes::ENote::{self, *}};
    use scale::{init_scale, EScale};

    #[inline]
    fn test_scale(root: ENote, scale: EScale, expected: &[ENote]) {
        let result = init_scale(root, scale);
        assert_eq!(result, expected);
    }

    #[test]
    fn c_major() {
        test_scale(C, EScale::Major, &[C, D, E, F, G, A, B]);
    }

    #[test]
    fn d_major() {
        test_scale(D, EScale::Major, &[D, E, Fs, G, A, B, Cs]);
    }

    #[test]
    fn g_major() {
        test_scale(G, EScale::Major, &[G, A, B, C, D, E, Fs]);
    }

    #[test]
    fn b_major() {
        test_scale(B, EScale::Major, &[B, Cs, Ds, E, Fs, Gs, As]);
    }

    #[test]
    fn c_minor() {
        test_scale(C, EScale::MinorN, &[C, D, Ds, F, G, Gs, As]);
    }

    #[test]
    fn d_minor() {
        test_scale(D, EScale::MinorN, &[D, E, F, G, A, As, C]);
    }

    #[test]
    fn g_minor() {
        test_scale(G, EScale::MinorN, &[G, A, As, C, D, Ds, F]);
    }

    #[test]
    fn b_minor() {
        test_scale(B, EScale::MinorN, &[B, Cs, D, E, Fs, G, A]);
    }
}
