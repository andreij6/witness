use candid::{CandidType, Deserialize};
use std::fmt::Write;

pub const BOOT: &str = "boot.mjs";
pub const MAX_PINS: usize = 256;
pub const MAX_PATH: usize = 128;
const SRI_PREFIX: &str = "sha384-";
const SRI_DIGEST_CHARS: usize = 64;

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    pub path: String,
    pub sri: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pins(Vec<Pin>);

fn path_ok(path: &str) -> bool {
    (1..=MAX_PATH).contains(&path.len())
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/'))
        && path
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != "..")
}

fn sri_ok(sri: &str) -> bool {
    sri.strip_prefix(SRI_PREFIX).is_some_and(|d| {
        d.len() == SRI_DIGEST_CHARS
            && d.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
    })
}

pub fn json_str_into(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' | '>' | '&' | '\u{2028}' | '\u{2029}' | '\0'..='\u{1f}' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            _ => out.push(c),
        }
    }
    out.push('"');
}

impl Pins {
    pub fn new(mut pins: Vec<Pin>) -> Result<Self, String> {
        if pins.len() > MAX_PINS {
            return Err(format!("pins: at most {MAX_PINS} entries"));
        }
        if let Some(p) = pins.iter().find(|p| !path_ok(&p.path)) {
            return Err(format!("pins: {:?} is not a safe relative path", p.path));
        }
        if let Some(p) = pins.iter().find(|p| !sri_ok(&p.sri)) {
            return Err(format!("pins: {:?} has no sha384 integrity", p.path));
        }
        pins.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        if let Some(w) = pins.windows(2).find(|w| w[0].path == w[1].path) {
            return Err(format!("pins: {:?} is pinned twice", w[0].path));
        }
        let pins = Self(pins);
        if pins.boot().is_none() {
            return Err(format!("pins: {BOOT} must be pinned"));
        }
        Ok(pins)
    }

    pub fn from_arg(arg: Option<Vec<Pin>>) -> Result<Self, String> {
        arg.map_or_else(|| Ok(Self::default()), Self::new)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, path: &str) -> Option<&str> {
        self.0
            .binary_search_by(|p| p.path.as_str().cmp(path))
            .ok()
            .map(|i| self.0[i].sri.as_str())
    }

    pub fn boot(&self) -> Option<&str> {
        self.get(BOOT)
    }

    pub fn import_map(&self, app: &str) -> String {
        let mut out = String::with_capacity(16 + self.0.len() * (app.len() + 96));
        out.push_str("{\"integrity\":{");
        for (i, p) in self.0.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            json_str_into(&mut out, &format!("{app}/{}", p.path));
            out.push(':');
            json_str_into(&mut out, &p.sri);
        }
        out.push_str("}}");
        out
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const SRI_A: &str =
        "sha384-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    pub(crate) const SRI_B: &str =
        "sha384-oqVuAfXRKap7fdgcCY5uykM6+R9GqQ8K/uxy9rx7HNQlGYl1kPzQho1wx4JwY8wC";

    pub(crate) fn pin(path: &str, sri: &str) -> Pin {
        Pin {
            path: path.into(),
            sri: sri.into(),
        }
    }

    pub(crate) fn sample() -> Pins {
        Pins::new(vec![
            pin("witness-config.mjs", SRI_A),
            pin(BOOT, SRI_B),
            pin("658820f72b4617efb380.wasm", SRI_A),
        ])
        .unwrap()
    }

    #[test]
    fn accepts_sorts_and_looks_up() {
        let p = sample();
        assert_eq!(p.len(), 3);
        assert!(!p.is_empty());
        assert_eq!(p.boot(), Some(SRI_B));
        assert_eq!(p.get("witness-config.mjs"), Some(SRI_A));
        assert_eq!(p.get("missing.mjs"), None);
        let paths: Vec<&str> = p.0.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(
            paths,
            ["658820f72b4617efb380.wasm", BOOT, "witness-config.mjs"]
        );
    }

    #[test]
    fn order_of_input_does_not_matter() {
        let a = Pins::new(vec![pin(BOOT, SRI_B), pin("a.js", SRI_A)]).unwrap();
        let b = Pins::new(vec![pin("a.js", SRI_A), pin(BOOT, SRI_B)]).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.import_map("https://x"), b.import_map("https://x"));
    }

    #[test]
    fn absent_arg_is_unpinned() {
        let p = Pins::from_arg(None).unwrap();
        assert!(p.is_empty());
        assert_eq!(p.boot(), None);
        assert_eq!(p.import_map("https://x"), "{\"integrity\":{}}");
    }

    #[test]
    fn boot_must_be_pinned() {
        assert!(Pins::from_arg(Some(Vec::new())).is_err());
        assert!(Pins::new(vec![pin("a.js", SRI_A)]).is_err());
        assert!(Pins::new(vec![pin("sub/boot.mjs", SRI_A)]).is_err());
    }

    #[test]
    fn rejects_unsafe_paths() {
        let long = "a".repeat(MAX_PATH + 1);
        for bad in [
            "",
            "/boot.mjs",
            "a//b.js",
            "a/",
            "../x.js",
            "a/../x.js",
            "./x.js",
            ".",
            "a b.js",
            "a\"b.js",
            "a<b.js",
            "a?b.js",
            "a#b.js",
            "a%2e.js",
            "a\\b.js",
            "ünï.js",
            long.as_str(),
        ] {
            assert!(
                Pins::new(vec![pin(BOOT, SRI_A), pin(bad, SRI_A)]).is_err(),
                "{bad:?}"
            );
        }
        let max = "a".repeat(MAX_PATH);
        assert!(Pins::new(vec![pin(BOOT, SRI_A), pin(&max, SRI_A)]).is_ok());
        assert!(
            Pins::new(vec![
                pin(BOOT, SRI_A),
                pin("composeResources/x.y_z-1.js", SRI_A)
            ])
            .is_ok()
        );
    }

    #[test]
    fn rejects_malformed_integrity() {
        let short = &SRI_A[..SRI_A.len() - 1];
        let long = format!("{SRI_A}A");
        let padded = format!("{}=", &SRI_A[..SRI_A.len() - 1]);
        let sha256 = SRI_A.replace("sha384", "sha256");
        let upper = SRI_A.replace("sha384", "SHA384");
        let two = format!("{SRI_A} {SRI_B}");
        let quote = format!("{}\"", &SRI_A[..SRI_A.len() - 1]);
        let url_safe = format!("{}-", &SRI_A[..SRI_A.len() - 1]);
        for bad in [
            "", "sha384-", short, &long, &padded, &sha256, &upper, &two, &quote, &url_safe,
        ] {
            assert!(Pins::new(vec![pin(BOOT, bad)]).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn rejects_duplicates_and_too_many() {
        assert!(Pins::new(vec![pin(BOOT, SRI_A), pin(BOOT, SRI_B)]).is_err());
        let mut many: Vec<Pin> = (0..MAX_PINS)
            .map(|i| pin(&format!("m{i}.mjs"), SRI_A))
            .collect();
        many[0] = pin(BOOT, SRI_A);
        assert_eq!(Pins::new(many.clone()).unwrap().len(), MAX_PINS);
        many.push(pin("extra.mjs", SRI_A));
        assert!(Pins::new(many).is_err());
    }

    #[test]
    fn import_map_is_json_keyed_by_absolute_url() {
        let m = sample().import_map("http://web-id.localhost:8090");
        assert_eq!(
            m,
            format!(
                "{{\"integrity\":{{\"http://web-id.localhost:8090/658820f72b4617efb380.wasm\":\"{SRI_A}\",\"http://web-id.localhost:8090/boot.mjs\":\"{SRI_B}\",\"http://web-id.localhost:8090/witness-config.mjs\":\"{SRI_A}\"}}}}"
            )
        );
        assert!(!m.contains('<'));
    }

    #[test]
    fn json_strings_cannot_break_out_of_a_script_element() {
        let mut s = String::new();
        json_str_into(&mut s, "</script><!--\"\\&\n\u{2028}");
        assert_eq!(
            s,
            "\"\\u003c/script\\u003e\\u003c!--\\\"\\\\\\u0026\\u000a\\u2028\""
        );
        let hostile = sample().import_map("https://a\"</script><script>alert(1)//");
        assert!(!hostile.contains("</script"));
        assert!(!hostile.contains("<script"));
    }

    #[test]
    fn candid_round_trip() {
        let arg = Some(vec![pin(BOOT, SRI_A)]);
        let bytes = candid::encode_one(&arg).unwrap();
        let back: Option<Vec<Pin>> = candid::decode_one(&bytes).unwrap();
        assert_eq!(back, arg);
    }
}
