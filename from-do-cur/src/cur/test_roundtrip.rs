#[cfg(test)]
mod tests {
    use super::super::{Phrase, strfcur, strpcur};
    use jiff::{ToSpan, Zoned};

    #[test]
    fn roundtrip_days() {
        let reference = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();
        let from = reference.saturating_sub(1.year());
        let to = reference.saturating_add(1.year());

        let mut current = from;
        loop {
            let u = Phrase::unresolve(&current, &reference);
            let f = strfcur(&u);
            let p = strpcur(&f).unwrap();
            let r = p.resolve(&reference);

            assert_eq!(current, r, "{current} -> {f} -> {r}");

            if current == to {
                break;
            }
            current = current.saturating_add(1.day());
        }
    }

    #[test]
    fn roundtrip_secs() {
        let reference = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();
        let from = reference.saturating_sub(1.day());
        let to = reference.saturating_add(1.day());

        let mut current = from;
        loop {
            let u = Phrase::unresolve(&current, &reference);
            let f = strfcur(&u);
            let p = strpcur(&f).unwrap();
            let r = p.resolve(&reference);

            assert_eq!(current, r, "{current} -> {f} -> {r}");

            if current == to {
                break;
            }
            current = current.saturating_add(1.second());
        }
    }
}
