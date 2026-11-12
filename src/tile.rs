//! The Dock tile artwork per face, drawn from Haunt's app icon (haunt/design/icon/haunt.svg): the
//! same squircle, three panes and the ghost pane, with only the eyes changing.
//! Needs-you eyes are the open ones, in orange, painted on the ghost rather than over a hole, so
//! no dark edge shows round them.

pub const ORANGE: &str = "#D97757";

/// Which drawing a face uses; the badge count is the host's, not part of the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Art {
    Sleepy,
    Open,
    /// The open eyes mid-blink.
    Blink,
    Needs,
}

impl Art {
    pub const ALL: [Art; 4] = [Art::Sleepy, Art::Open, Art::Blink, Art::Needs];

    pub fn file(self) -> &'static str {
        match self {
            Art::Sleepy => "sleepy",
            Art::Open => "open",
            Art::Blink => "blink",
            Art::Needs => "needs",
        }
    }
}

/// Sleepy eyes are the icon's half-ellipse cut-outs; open eyes are full ellipses.
fn eye_cutouts(art: Art) -> &'static str {
    match art {
        Art::Sleepy => r##"<path d="M580 640 A40 30 0 0 0 660 640 Z" fill="#000"/><path d="M668 640 A40 30 0 0 0 748 640 Z" fill="#000"/>"##,
        Art::Needs => "",
        Art::Open => r##"<ellipse cx="620" cy="650" rx="21" ry="25" fill="#000"/><ellipse cx="708" cy="650" rx="21" ry="25" fill="#000"/>"##,
        Art::Blink => r##"<ellipse cx="620" cy="650" rx="21" ry="2.5" fill="#000"/><ellipse cx="708" cy="650" rx="21" ry="2.5" fill="#000"/>"##,
    }
}

pub fn svg(art: Art) -> String {
    let needs = art == Art::Needs;
    let orange_eyes = if needs {
        format!(r##"<ellipse cx="620" cy="650" rx="21" ry="25" fill="{ORANGE}"/><ellipse cx="708" cy="650" rx="21" ry="25" fill="{ORANGE}"/>"##)
    } else {
        String::new()
    };
    format!(
        r##"<svg viewBox="0 0 1024 1024" width="1024" height="1024" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <radialGradient id="hbg" cx=".5" cy=".38" r=".8"><stop offset="0" stop-color="#2e2e2e"/><stop offset="1" stop-color="#0d0d0d"/></radialGradient>
    <linearGradient id="hink" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#f0f0f0"/><stop offset="1" stop-color="#bdbdbd"/></linearGradient>
    <filter id="hblur"><feGaussianBlur stdDeviation="46"/></filter>
    <clipPath id="hclip"><rect width="1024" height="1024" rx="230"/></clipPath>
    <mask id="hm"><rect width="1024" height="1024" fill="#fff"/>{eyes}</mask>
  </defs>
  <g transform="translate(100 100) scale(.8047)">
    <g clip-path="url(#hclip)">
      <rect width="1024" height="1024" fill="url(#hbg)"/>
      <circle cx="512" cy="400" r="380" fill="#5a5a5a" opacity=".22" filter="url(#hblur)"/>
      <g fill="url(#hink)" opacity=".55">
        <rect x="220" y="220" width="280" height="280" rx="40"/><rect x="524" y="220" width="280" height="280" rx="40"/>
        <rect x="220" y="524" width="280" height="280" rx="40"/>
      </g>
      <path d="M524 820 V564 Q524 524 564 524 H764 Q804 524 804 564 V820 Q757.333 776 710.667 820 Q664 776 617.333 820 Q570.667 776 524 820 Z" fill="#e8e8e8" mask="url(#hm)"/>
      {orange_eyes}
    </g>
    <rect width="1024" height="1024" rx="230" fill="none" stroke="#fff" stroke-opacity=".14" stroke-width="4"/>
  </g>
</svg>
"##,
        eyes = eye_cutouts(art),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleepy_is_the_app_icon() {
        let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/../../haunt/design/icon/haunt.svg");
        let Ok(icon) = std::fs::read_to_string(icon) else { return };
        let geometry = |s: &str| -> Vec<String> { s.lines().filter(|l| l.contains("<rect") || l.contains("<path") || l.contains("<mask")).map(|l| l.trim().to_owned()).collect() };
        assert_eq!(geometry(&svg(Art::Sleepy)), geometry(&icon));
    }

    #[test]
    fn needs_is_the_open_face_in_orange_and_nothing_else() {
        let (needs, open) = (svg(Art::Needs), svg(Art::Open));
        let holes = eye_cutouts(Art::Open);
        assert_eq!(needs.replace(&format!(r##"<ellipse cx="620" cy="650" rx="21" ry="25" fill="{ORANGE}"/><ellipse cx="708" cy="650" rx="21" ry="25" fill="{ORANGE}"/>"##), ""), open.replace(holes, ""));
        assert_eq!(holes.replace("#000", ORANGE), format!(r##"<ellipse cx="620" cy="650" rx="21" ry="25" fill="{ORANGE}"/><ellipse cx="708" cy="650" rx="21" ry="25" fill="{ORANGE}"/>"##), "the open eyes' shape and place");
        assert!(!needs.contains("stroke=\"#D97757\""), "no outline");
    }

    #[test]
    fn only_needs_is_orange() {
        for a in Art::ALL {
            assert_eq!(svg(a).contains(ORANGE), a == Art::Needs, "{a:?}");
        }
    }

    #[test]
    fn assets_match_the_drawing() {
        for a in Art::ALL {
            let path = format!("{}/assets/{}.svg", env!("CARGO_MANIFEST_DIR"), a.file());
            if std::env::var_os("REGEN_ASSETS").is_some() {
                std::fs::write(&path, svg(a)).unwrap();
            }
            assert_eq!(std::fs::read_to_string(&path).unwrap_or_default(), svg(a), "{path} is stale: rerun with REGEN_ASSETS=1");
        }
    }

    #[test]
    fn the_dock_gets_512_pixel_pngs_under_a_megabyte() {
        for a in Art::ALL {
            let path = format!("{}/assets/{}.png", env!("CARGO_MANIFEST_DIR"), a.file());
            let png = std::fs::read(&path).unwrap();
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "{path}");
            let size = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
            assert_eq!((size(16), size(20)), (512, 512), "{path}");
            assert!(png.len() < 1_000_000, "{path}");
        }
    }
}
