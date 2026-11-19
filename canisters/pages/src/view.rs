use witness_format::{Edit, EditKind, Geohash5, Kind};

const CHANGE_ORDER: [(EditKind, &str, &str); 5] = [
    (EditKind::Crop, "a crop", "crops"),
    (EditKind::Rotate, "a rotation", "rotations"),
    (EditKind::Tone, "a tone adjustment", "tone adjustments"),
    (EditKind::Filter, "a filter", "filters"),
    (EditKind::Blur, "a declared blur", "declared blurs"),
];

pub fn changes(edits: &[Edit]) -> Option<String> {
    let parts: Vec<String> = CHANGE_ORDER
        .iter()
        .filter_map(
            |(kind, one, many)| match edits.iter().filter(|e| e.kind() == *kind).count() {
                0 => None,
                1 => Some((*one).to_string()),
                n => Some(format!("{n} {many}")),
            },
        )
        .collect();
    match parts.as_slice() {
        [] => None,
        [one] => Some(one.clone()),
        [init @ .., last] => Some(format!("{} and {last}", init.join(", "))),
    }
}

pub fn headline(kind: &Kind) -> String {
    match kind {
        Kind::Original => "Sealed original".to_string(),
        Kind::Derivative { edits, .. } => match changes(edits) {
            None => "Matches an original".to_string(),
            Some(c) => format!("Matches an original, with {c}"),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utc {
    pub iso: String,
    pub human: String,
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

pub fn utc(ms: u64) -> Utc {
    let secs = ms / 1000;
    let (y, m, d) = civil((secs / 86_400) as i64);
    let sod = secs % 86_400;
    let (hh, mm, ss) = (sod / 3600, sod % 3600 / 60, sod % 60);
    Utc {
        iso: format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z"),
        human: format!("{d} {} {y}, {hh:02}:{mm:02} UTC", MONTHS[(m - 1) as usize]),
    }
}

const NANO: i64 = 1_000_000_000;

fn degrees_tenths(v: i64) -> i64 {
    let half = NANO / 20;
    (v + v.signum() * half) / (NANO / 10)
}

fn fmt_deg(tenths: i64, pos: char, neg: char) -> String {
    let a = tenths.abs();
    let hemi = if tenths < 0 { neg } else { pos };
    format!("{}.{}° {hemi}", a / 10, a % 10)
}

pub fn geohash_center(g: &Geohash5) -> (i64, i64) {
    let (mut lat, mut lon) = ([-90 * NANO, 90 * NANO], [-180 * NANO, 180 * NANO]);
    let mut even = true;
    for c in g.as_str().bytes() {
        let idx = Geohash5::ALPHABET.iter().position(|a| *a == c).unwrap_or(0);
        for b in (0..5).rev() {
            let r = if even { &mut lon } else { &mut lat };
            let mid = (r[0] + r[1]) / 2;
            r[usize::from((idx >> b) & 1 == 0)] = mid;
            even = !even;
        }
    }
    ((lat[0] + lat[1]) / 2, (lon[0] + lon[1]) / 2)
}

pub fn area(g: &Geohash5) -> String {
    let (lat, lon) = geohash_center(g);
    format!(
        "near {}, {}",
        fmt_deg(degrees_tenths(lat), 'N', 'S'),
        fmt_deg(degrees_tenths(lon), 'E', 'W')
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use witness_format::{BlurClass, CaptureId, ImageFormat, RecordId, Rect};

    fn blur() -> Edit {
        Edit::Blur {
            rect: Rect {
                x: 0,
                y: 0,
                w: 2,
                h: 2,
            },
            class: BlurClass::Face,
            sigma_e2: 100,
        }
    }

    fn crop() -> Edit {
        Edit::Crop(Rect {
            x: 0,
            y: 0,
            w: 5,
            h: 5,
        })
    }

    fn derivative(edits: Vec<Edit>) -> Kind {
        Kind::Derivative {
            parent: RecordId::from_bytes([1; 10]),
            root: CaptureId::new(1, 0).unwrap(),
            edits,
        }
    }

    #[test]
    fn changes_ignores_presentation_edits() {
        let e = [
            Edit::Resize {
                w: 10,
                h: 10,
                filter: 0,
            },
            Edit::Recompress {
                format: ImageFormat::Jpeg,
                quality: 80,
            },
            Edit::Orient { orientation: 1 },
        ];
        assert_eq!(changes(&e), None);
        assert_eq!(changes(&[]), None);
    }

    #[test]
    fn changes_pluralises_and_joins_in_fixed_order() {
        assert_eq!(changes(&[blur()]).unwrap(), "a declared blur");
        assert_eq!(changes(&[blur(), blur()]).unwrap(), "2 declared blurs");
        assert_eq!(
            changes(&[blur(), crop(), blur()]).unwrap(),
            "a crop and 2 declared blurs"
        );
        let all = [
            blur(),
            Edit::Filter {
                id: 1,
                strength_e2: 50,
            },
            Edit::Tone {
                exposure_e2: 1,
                contrast_e2: 0,
                saturation_e2: 0,
            },
            Edit::Rotate { deg_e2: 100 },
            crop(),
            crop(),
        ];
        assert_eq!(
            changes(&all).unwrap(),
            "2 crops, a rotation, a tone adjustment, a filter and a declared blur"
        );
    }

    #[test]
    fn headline_by_kind() {
        assert_eq!(headline(&Kind::Original), "Sealed original");
        assert_eq!(headline(&derivative(vec![])), "Matches an original");
        assert_eq!(
            headline(&derivative(vec![blur(), blur()])),
            "Matches an original, with 2 declared blurs"
        );
    }

    #[test]
    fn utc_formats_known_instants() {
        assert_eq!(
            utc(0),
            Utc {
                iso: "1970-01-01T00:00:00Z".into(),
                human: "1 Jan 1970, 00:00 UTC".into()
            }
        );
        assert_eq!(utc(1_700_000_000_000).iso, "2023-11-14T22:13:20Z");
        assert_eq!(utc(951_782_400_000).iso, "2000-02-29T00:00:00Z");
        assert_eq!(utc(1_795_132_799_999).human, "19 Nov 2026, 23:59 UTC");
        assert_eq!(utc(4_107_542_400_000).iso, "2100-03-01T00:00:00Z");
    }

    #[test]
    fn utc_handles_extremes_without_panicking() {
        let max = utc(u64::MAX);
        assert!(max.iso.ends_with('Z'));
        assert_eq!(utc(999).iso, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn utc_day_boundaries_roll_over() {
        let mut prev = utc(0).iso;
        for day in 1..3000u64 {
            let cur = utc(day * 86_400_000).iso;
            assert!(cur > prev, "{cur} after {prev}");
            assert!(cur.ends_with("T00:00:00Z"));
            prev = cur;
        }
    }

    #[test]
    fn geohash_known_cells() {
        let sf = Geohash5::new("9q8yy").unwrap();
        assert_eq!(area(&sf), "near 37.8° N, 122.4° W");
        let lon = Geohash5::new("gcpvj").unwrap();
        assert_eq!(area(&lon), "near 51.5° N, 0.1° W");
        let syd = Geohash5::new("r3gx2").unwrap();
        assert_eq!(area(&syd), "near 33.9° S, 151.2° E");
    }

    #[test]
    fn geohash_extremes() {
        let lo = Geohash5::new("00000").unwrap();
        let hi = Geohash5::new("zzzzz").unwrap();
        let (lat, lon) = geohash_center(&lo);
        assert!(lat < -89 * NANO && lon < -179 * NANO);
        let (lat, lon) = geohash_center(&hi);
        assert!(lat > 89 * NANO && lon > 179 * NANO);
        assert_eq!(area(&lo), "near 90.0° S, 180.0° W");
    }

    #[test]
    fn rounding_is_symmetric() {
        assert_eq!(degrees_tenths(NANO / 20), 1);
        assert_eq!(degrees_tenths(-NANO / 20), -1);
        assert_eq!(degrees_tenths(NANO / 20 - 1), 0);
        assert_eq!(degrees_tenths(0), 0);
        assert_eq!(fmt_deg(0, 'N', 'S'), "0.0° N");
        assert_eq!(fmt_deg(-5, 'E', 'W'), "0.5° W");
    }
}
