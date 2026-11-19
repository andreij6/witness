use crate::pins::Pins;
use crate::view::Utc;
use std::fmt::Write;
use std::sync::LazyLock;
use witness_canister_common::{Origins, Removal};
use witness_format::crypto::sha256;

pub fn ic_domains(o: &Origins) -> String {
    let site = Origins::host(&o.site);
    let case = Origins::host(&o.case);
    if site == case {
        format!("{site}\n")
    } else {
        format!("{site}\n{case}\n")
    }
}

const STYLE: &str = "<style>body{margin:0;font:16px/1.5 system-ui,sans-serif;color:#111;background:#fff}main{max-width:40rem;margin:0 auto;padding:2rem 1rem}h1{font-size:1.5rem;line-height:1.25}dt{font-weight:600}dd{margin:0 0 .75rem}.dev{display:inline-block;padding:.1rem .5rem;border-radius:.25rem;background:#b00020;color:#fff;font-weight:700}a{color:#0b57d0}@media(prefers-color-scheme:dark){body{color:#eee;background:#111}a{color:#8ab4f8}}</style>";

pub fn escape_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    escape_into(&mut out, s);
    out
}

pub struct ShellView<'a> {
    pub record_id: &'a str,
    pub grouped: &'a str,
    pub media: &'a str,
    pub media_shard: u8,
    pub headline: &'a str,
    pub changes: Option<&'a str>,
    pub original: bool,
    pub captured: &'a Utc,
    pub anchored: &'a Utc,
    pub area: Option<&'a str>,
    pub dev: bool,
}

impl ShellView<'_> {
    fn title(&self) -> String {
        if self.dev {
            format!("[DEV] {}", self.headline)
        } else {
            self.headline.to_string()
        }
    }

    fn description(&self) -> String {
        let mut d = format!("{}. Captured {}", self.headline, self.captured.human);
        if let Some(a) = self.area {
            d.push_str(", ");
            d.push_str(a);
        }
        d.push_str(". Checked against a public record that can't be edited.");
        d
    }

    fn proves(&self) -> String {
        let device = if self.dev {
            "a development device"
        } else {
            "a verified device"
        };
        let mut p = if self.original {
            format!("A Witness camera sealed this photo on {device} at the moment of capture.")
        } else {
            format!(
                "This is an edited copy of a photo that a Witness camera sealed on {device} at the moment of capture."
            )
        };
        match (self.original, self.changes) {
            (true, _) => {}
            (false, None) => p.push_str(" It was only resized or re-encoded."),
            (false, Some(c)) => {
                p.push_str(" Every change from the original is declared and signed: ");
                p.push_str(c);
                p.push('.');
            }
        }
        p.push_str(" The seal and its time are anchored in a public registry, so neither can be changed later. It doesn't show what happened outside the frame.");
        p
    }
}

fn meta(out: &mut String, attr: &str, name: &str, content: &str) {
    let _ = write!(out, "<meta {attr}=\"{name}\" content=\"");
    escape_into(out, content);
    out.push_str("\">");
}

fn dd(out: &mut String, label: &str, value: &str) {
    out.push_str("<dt>");
    out.push_str(label);
    out.push_str("</dt><dd>");
    escape_into(out, value);
    out.push_str("</dd>");
}

pub fn proof_shell(v: &ShellView, o: &Origins) -> String {
    let id = escape(v.record_id);
    let app = o.app.as_str();
    let url = format!("{}/p/{id}", o.site);
    let image = format!("{}/o/{id}", o.canister_url(&escape(v.media)));
    let title = v.title();
    let description = v.description();
    let mut o = String::with_capacity(4096);
    o.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>");
    escape_into(&mut o, &title);
    o.push_str(" · Witness</title>");
    meta(&mut o, "name", "description", &description);
    if v.dev {
        meta(&mut o, "name", "robots", "noindex, nofollow");
    }
    let _ = write!(o, "<link rel=\"canonical\" href=\"{url}\">");
    meta(&mut o, "property", "og:type", "article");
    meta(&mut o, "property", "og:site_name", "Witness");
    meta(&mut o, "property", "og:title", &title);
    meta(&mut o, "property", "og:description", &description);
    meta(&mut o, "property", "og:url", &url);
    meta(&mut o, "property", "og:image", &image);
    meta(&mut o, "property", "og:image:width", "1200");
    meta(&mut o, "property", "og:image:height", "630");
    meta(&mut o, "name", "twitter:card", "summary_large_image");
    meta(&mut o, "name", "twitter:title", &title);
    meta(&mut o, "name", "twitter:description", &description);
    meta(&mut o, "name", "twitter:image", &image);
    o.push_str(STYLE);
    o.push_str("</head><body><main>");
    if v.dev {
        o.push_str("<p class=\"dev\">DEV</p>");
    }
    o.push_str("<h1>");
    escape_into(&mut o, v.headline);
    o.push_str("</h1><dl><dt>Captured</dt><dd><time datetime=\"");
    escape_into(&mut o, &v.captured.iso);
    o.push_str("\">");
    escape_into(&mut o, &v.captured.human);
    o.push_str("</time></dd><dt>First seen</dt><dd><time datetime=\"");
    escape_into(&mut o, &v.anchored.iso);
    o.push_str("\">");
    escape_into(&mut o, &v.anchored.human);
    o.push_str("</time></dd>");
    if let Some(a) = v.area {
        dd(&mut o, "Area", a);
    }
    dd(&mut o, "Record", v.grouped);
    o.push_str("</dl><h2>What this proves</h2><p>");
    escape_into(&mut o, &v.proves());
    let _ = write!(
        o,
        "</p><p><a href=\"{app}/report/{id}\">Report this page</a> · <a href=\"{app}/contact\">Contact</a></p></main>"
    );
    let _ = write!(
        o,
        "<script type=\"application/json\" id=\"witness-shell\">{{\"record_id\":\"{id}\",\"media_shard\":{}}}</script>",
        v.media_shard
    );
    let _ = write!(
        o,
        "<div id=\"app\"></div><script type=\"module\" src=\"{app}/boot.mjs\"></script></body></html>"
    );
    o
}

pub static CASE_NONCE: LazyLock<String> = LazyLock::new(|| {
    let h = sha256(concat!("witness/case-nonce/v1/", env!("CARGO_PKG_VERSION")).as_bytes());
    h[..16].iter().fold(String::with_capacity(32), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
});

pub fn case_shell(o: &Origins, pins: &Pins) -> String {
    let app = o.app.as_str();
    let nonce = CASE_NONCE.as_str();
    let mut h = String::with_capacity(1536 + pins.len() * (app.len() + 96));
    h.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"robots\" content=\"noindex, nofollow\"><meta name=\"referrer\" content=\"no-referrer\"><title>Shared case · Witness</title><meta property=\"og:title\" content=\"Shared case\"><meta property=\"og:description\" content=\"A private case shared with Witness.\">");
    if !pins.is_empty() {
        let _ = write!(
            h,
            "<script type=\"importmap\" nonce=\"{nonce}\">{}</script>",
            pins.import_map(app)
        );
    }
    h.push_str(STYLE);
    h.push_str("</head><body><main><h1>Shared case</h1><p>This case is encrypted. The key is in the link you were sent and never reaches a server.</p><noscript><p>Turn on JavaScript to open this case.</p></noscript></main><div id=\"app\"></div><script type=\"module\" nonce=\"");
    h.push_str(nonce);
    h.push('"');
    if let Some(sri) = pins.boot() {
        h.push_str(" integrity=\"");
        escape_into(&mut h, sri);
        h.push_str("\" crossorigin=\"anonymous\"");
    }
    let _ = write!(h, " src=\"{app}/boot.mjs\"></script></body></html>");
    h
}

fn plain(title: &str, body: &str, o: &Origins) -> String {
    let app = o.app.as_str();
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"robots\" content=\"noindex, nofollow\"><title>{title} · Witness</title>{STYLE}</head><body><main><h1>{title}</h1><p>{body}</p><p><a href=\"{app}/check\">Check a photo</a> · <a href=\"{app}/contact\">Contact</a></p></main></body></html>"
    )
}

pub fn not_found(o: &Origins) -> String {
    plain(
        "No proof page here",
        "There is no Witness proof page at this address. Check the link, or check the photo itself.",
        o,
    )
}

pub fn gone(why: Removal, o: &Origins) -> String {
    let (title, body) = match why {
        Removal::Erased => (
            "Page removed by its owner",
            "The person who posted this removed the page. The permanent record stays in the registry, but its preview and details are gone.",
        ),
        Removal::Takedown => (
            "Page taken down",
            "This page was taken down after a report. The permanent record stays in the registry, but its preview and details are gone.",
        ),
        Removal::Revoked => (
            "Page withdrawn",
            "The device key that sealed this photo was revoked, so this page no longer vouches for it.",
        ),
    };
    plain(title, body, o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::utc;

    fn prod() -> Origins {
        Origins::default()
    }

    fn local() -> Origins {
        Origins {
            site: "http://pages-id.localhost:8090".into(),
            case: "http://pages-id.localhost:8090".into(),
            app: "http://web-id.localhost:8090".into(),
            api: "http://localhost:8090".into(),
            canister: "http://*.localhost:8090".into(),
        }
    }

    fn view<'a>(
        captured: &'a Utc,
        area: Option<&'a str>,
        dev: bool,
        headline: &'a str,
    ) -> ShellView<'a> {
        ShellView {
            record_id: "0123456789ABCDEF",
            grouped: "0123-4567-89AB-CDEF",
            media: "aaaaa-aa",
            media_shard: 3,
            headline,
            changes: Some("2 declared blurs"),
            original: false,
            captured,
            anchored: captured,
            area,
            dev,
        }
    }

    fn proof_shell_prod(v: &ShellView) -> String {
        proof_shell(v, &prod())
    }

    fn count(h: &str, needle: &str) -> usize {
        h.matches(needle).count()
    }

    #[test]
    fn escape_covers_all_specials() {
        assert_eq!(
            escape("<a href=\"x\" onclick='y'>&</a>"),
            "&lt;a href=&quot;x&quot; onclick=&#39;y&#39;&gt;&amp;&lt;/a&gt;"
        );
        assert_eq!(escape(""), "");
        assert_eq!(escape("plain ünïcode ✓"), "plain ünïcode ✓");
        assert_eq!(escape("&amp;"), "&amp;amp;");
    }

    #[test]
    fn escape_into_appends() {
        let mut s = String::from("x");
        escape_into(&mut s, "<");
        assert_eq!(s, "x&lt;");
    }

    #[test]
    fn proof_shell_has_required_structure() {
        let t = utc(1_700_000_000_000);
        let h = proof_shell_prod(&view(
            &t,
            Some("near 37.8° N, 122.4° W"),
            false,
            "Matches an original, with 2 declared blurs",
        ));
        assert!(h.starts_with("<!doctype html>"));
        assert!(h.ends_with("</html>"));
        assert!(
            h.contains("<link rel=\"canonical\" href=\"https://witness.app/p/0123456789ABCDEF\">")
        );
        assert!(h.contains(
            "<meta property=\"og:image\" content=\"https://aaaaa-aa.icp0.io/o/0123456789ABCDEF\">"
        ));
        assert!(h.contains("<meta name=\"twitter:card\" content=\"summary_large_image\">"));
        assert!(h.contains("{\"record_id\":\"0123456789ABCDEF\",\"media_shard\":3}"));
        assert!(h.contains("<div id=\"app\"></div><script type=\"module\" src=\"https://app.witness.app/boot.mjs\"></script>"));
        assert!(
            h.contains("<time datetime=\"2023-11-14T22:13:20Z\">14 Nov 2023, 22:13 UTC</time>")
        );
        assert!(h.contains("<dt>Area</dt><dd>near 37.8° N, 122.4° W</dd>"));
        assert!(h.contains("<dt>Record</dt><dd>0123-4567-89AB-CDEF</dd>"));
        assert!(h.contains("declared and signed: 2 declared blurs."));
        assert!(h.contains("sealed on a verified device"));
        assert!(!h.contains("development device"));
        assert!(h.contains("https://app.witness.app/report/0123456789ABCDEF"));
        assert!(!h.contains("noindex"));
        assert!(!h.contains("class=\"dev\""));
        assert_eq!(count(&h, "<main>"), 1);
        assert_eq!(count(&h, "<script"), 2);
        assert!(h.len() < 4096, "shell is {} bytes", h.len());
    }

    #[test]
    fn dev_shell_is_noindex_and_badged() {
        let t = utc(0);
        let h = proof_shell_prod(&view(&t, None, true, "Sealed original"));
        assert!(h.contains("<meta name=\"robots\" content=\"noindex, nofollow\">"));
        assert!(h.contains("<p class=\"dev\">DEV</p>"));
        assert!(h.contains("<title>[DEV] Sealed original · Witness</title>"));
        assert!(!h.contains("<dt>Area</dt>"));
        assert!(h.contains("sealed on a development device"));
        assert!(!h.contains("verified device"));
    }

    #[test]
    fn original_and_presentation_only_copy() {
        let t = utc(0);
        let mut v = view(&t, None, false, "Sealed original");
        v.original = true;
        assert!(!proof_shell(&v, &prod()).contains("declared and signed"));
        v.original = false;
        v.changes = None;
        assert!(proof_shell(&v, &prod()).contains("only resized or re-encoded"));
    }

    #[test]
    fn hostile_fields_are_escaped_everywhere() {
        let evil = "\"><script>alert(1)</script><x a='";
        let t = Utc {
            iso: evil.into(),
            human: evil.into(),
        };
        let v = ShellView {
            record_id: evil,
            grouped: evil,
            media: evil,
            media_shard: 0,
            headline: evil,
            changes: Some(evil),
            original: false,
            captured: &t,
            anchored: &t,
            area: Some(evil),
            dev: true,
        };
        let h = proof_shell(&v, &prod());
        assert_eq!(count(&h, "<script"), 2);
        assert!(!h.contains("alert(1)</script>"));
        assert!(!h.contains("'>"));
        assert!(h.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    }

    #[test]
    fn proof_shell_is_deterministic() {
        let t = utc(42_000);
        let a = proof_shell_prod(&view(&t, Some("near"), false, "H"));
        let b = proof_shell_prod(&view(&t, Some("near"), false, "H"));
        assert_eq!(a, b);
    }

    #[test]
    fn case_shell_is_generic_noindex_and_nonced() {
        let shell = case_shell(&prod(), &Pins::default());
        let h = shell.as_str();
        assert_eq!(CASE_NONCE.len(), 32);
        assert!(CASE_NONCE.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(h.contains(&format!("nonce=\"{}\"", CASE_NONCE.as_str())));
        assert!(h.contains("noindex, nofollow"));
        assert!(h.contains("no-referrer"));
        assert_eq!(count(h, "<script"), 1);
        assert!(!h.contains("record_id"));
    }

    #[test]
    fn pinned_case_shell_carries_an_integrity_import_map_and_boot_sri() {
        use crate::pins::tests::{SRI_B, sample};
        let pins = sample();
        let h = case_shell(&local(), &pins);
        let nonce = CASE_NONCE.as_str();
        let map = format!(
            "<script type=\"importmap\" nonce=\"{nonce}\">{}</script>",
            pins.import_map("http://web-id.localhost:8090")
        );
        assert!(h.contains(&map));
        assert!(h.contains("\"http://web-id.localhost:8090/boot.mjs\":"));
        assert!(h.contains("\"http://web-id.localhost:8090/witness-config.mjs\":"));
        assert!(h.contains(&format!(
            "<script type=\"module\" nonce=\"{nonce}\" integrity=\"{SRI_B}\" crossorigin=\"anonymous\" src=\"http://web-id.localhost:8090/boot.mjs\"></script>"
        )));
        assert!(h.find("type=\"importmap\"") < h.find("type=\"module\""));
        assert!(h.find("type=\"importmap\"") < h.find("</head>"));
        assert_eq!(count(&h, "<script"), 2);
        assert_eq!(count(&h, &format!("nonce=\"{nonce}\"")), 2);
        assert!(!h.contains("record_id"));
    }

    #[test]
    fn unpinned_case_shell_has_no_import_map_or_integrity() {
        let h = case_shell(&prod(), &Pins::default());
        assert!(!h.contains("importmap"));
        assert!(!h.contains("integrity"));
        assert!(!h.contains("crossorigin"));
        assert!(h.contains(&format!(
            "<script type=\"module\" nonce=\"{}\" src=\"https://app.witness.app/boot.mjs\"></script>",
            CASE_NONCE.as_str()
        )));
    }

    #[test]
    fn case_shell_import_map_is_escaped_inside_the_script_element() {
        use crate::pins::tests::sample;
        let o = Origins {
            app: "https://a</script><script>alert(1)//".into(),
            ..Origins::default()
        };
        let h = case_shell(&o, &sample());
        let head = &h[..h.find("</head>").unwrap()];
        assert_eq!(count(head, "</script>"), 1);
        assert!(head.contains("\\u003c/script\\u003e\\u003cscript\\u003ealert(1)"));
        let start = head.find("type=\"importmap\"").unwrap();
        let body = &head[start..head.find("</script>").unwrap()];
        assert!(!body[body.find('>').unwrap() + 1..].contains('<'));
    }

    #[test]
    fn pinned_case_shell_is_deterministic_and_input_order_free() {
        use crate::pins::tests::{SRI_A, SRI_B, pin};
        use crate::pins::{BOOT, Pins};
        let a = Pins::new(vec![pin(BOOT, SRI_B), pin("x.wasm", SRI_A)]).unwrap();
        let b = Pins::new(vec![pin("x.wasm", SRI_A), pin(BOOT, SRI_B)]).unwrap();
        assert_eq!(case_shell(&prod(), &a), case_shell(&prod(), &b));
        let c = Pins::new(vec![pin(BOOT, SRI_A), pin("x.wasm", SRI_A)]).unwrap();
        assert_ne!(case_shell(&prod(), &a), case_shell(&prod(), &c));
    }

    #[test]
    fn static_pages_are_noindex_and_scriptless() {
        let o = prod();
        for h in [
            not_found(&o),
            gone(Removal::Erased, &o),
            gone(Removal::Takedown, &o),
            gone(Removal::Revoked, &o),
        ] {
            assert!(h.contains("noindex"));
            assert!(!h.contains("<script"));
            assert!(h.contains("https://app.witness.app/check"));
        }
        assert_ne!(gone(Removal::Erased, &o), gone(Removal::Takedown, &o));
        assert_ne!(gone(Removal::Takedown, &o), gone(Removal::Revoked, &o));
    }

    #[test]
    fn ic_domains_lists_both_hosts() {
        assert_eq!(
            ic_domains(&prod()).lines().collect::<Vec<_>>(),
            ["witness.app", "case.witness.app"]
        );
    }

    #[test]
    fn local_origins_point_the_shell_at_the_local_web_canister() {
        let t = utc(0);
        let o = local();
        let h = proof_shell(&view(&t, None, false, "Sealed original"), &o);
        assert!(h.contains(
            "<script type=\"module\" src=\"http://web-id.localhost:8090/boot.mjs\"></script>"
        ));
        assert!(h.contains(
            "<link rel=\"canonical\" href=\"http://pages-id.localhost:8090/p/0123456789ABCDEF\">"
        ));
        assert!(h.contains("content=\"http://aaaaa-aa.localhost:8090/o/0123456789ABCDEF\""));
        assert!(h.contains("http://web-id.localhost:8090/report/0123456789ABCDEF"));
        assert!(!h.contains("witness.app"));
        assert!(!h.contains("icp0.io"));
        let c = case_shell(&o, &Pins::default());
        assert!(c.contains("src=\"http://web-id.localhost:8090/boot.mjs\""));
        assert!(!c.contains("witness.app"));
        for p in [not_found(&o), gone(Removal::Erased, &o)] {
            assert!(p.contains("http://web-id.localhost:8090/contact"));
            assert!(!p.contains("witness.app"));
        }
    }

    #[test]
    fn ic_domains_collapses_a_shared_host() {
        assert_eq!(ic_domains(&local()), "pages-id.localhost:8090\n");
        assert_eq!(ic_domains(&prod()), "witness.app\ncase.witness.app\n");
    }
}
