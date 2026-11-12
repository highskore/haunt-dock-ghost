//! Dock ghost: Haunt's Dock icon follows your agents. Asleep (the plain app icon) when nothing runs,
//! eyes open and blinking while agents work, and wide orange eyes with the red badge counting the
//! agents that need you.
// wit-bindgen's flattened `notify` import has 14 params.
#![allow(clippy::too_many_arguments)]

wit_bindgen::generate!({ path: "wit/haunt-plugin.wit", world: "plugin" });

// Shared with the other fleet-following plugins; not every helper is used here.
#[allow(dead_code)]
mod fleet;
mod ghost;
// The drawing itself is only run by the tests and assets.sh; the plugin ships the PNGs.
#[cfg_attr(not(test), allow(dead_code))]
mod tile;

use std::sync::Mutex;

use fleet::{Ev, Fleet};
use ghost::Face;
use haunt::plugin::dock;
use haunt::plugin::log::log;
use haunt::plugin::timer;
use haunt::plugin::types::LogLevel;
use serde::Deserialize;
use tile::Art;

const IMAGES: [(Art, &[u8]); 3] =
    [(Art::Open, include_bytes!("../assets/open.png")), (Art::Blink, include_bytes!("../assets/blink.png")), (Art::Needs, include_bytes!("../assets/needs.png"))];

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The red count of agents that need you.
    pub badge: bool,
    /// Working eyes blink now and then; never under Reduce Motion.
    pub blink: bool,
    pub unverified: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { badge: true, blink: true, unverified: false }
    }
}

#[derive(Default)]
struct Live {
    settings: Settings,
    fleet: Fleet,
    shown: Option<Tile>,
    /// The last refusal, logged once; the timer retries until a show goes through.
    refused: Option<String>,
}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Live) -> R) -> R {
    f(LIVE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(Live::default))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// What `dock.show` gets: an image id (none: Haunt's own icon), the badge and the blink frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    pub image: Option<&'static str>,
    pub badge: Option<u32>,
    pub blink: Option<&'static str>,
}

pub fn tile(face: Face, s: &Settings) -> Tile {
    match face {
        Face::Sleepy => Tile { image: None, badge: None, blink: None },
        Face::Working => Tile { image: Some(Art::Open.file()), badge: None, blink: s.blink.then(|| Art::Blink.file()) },
        Face::Awake(n) => Tile { image: Some(Art::Needs.file()), badge: s.badge.then_some(n), blink: None },
    }
}

fn show(l: &mut Live) {
    let t = tile(ghost::face(&l.fleet), &l.settings);
    if l.shown.as_ref() == Some(&t) {
        if l.refused.take().is_some() {
            timer::stop();
        }
        return;
    }
    match dock::show(t.image, t.badge, t.blink) {
        Ok(()) => {
            l.shown = Some(t);
            if l.refused.take().is_some() {
                timer::stop();
            }
        }
        // Too many shows a second, or another plugin has the tile.
        Err(e) => {
            if l.refused.as_ref() != Some(&e) {
                log(LogLevel::Warn, &format!("dock-ghost: tile not shown: {e}"));
                if l.refused.is_none() {
                    let _ = timer::every(15);
                }
                l.refused = Some(e);
            }
        }
    }
}

struct Plugin;

impl Guest for Plugin {
    fn init(json: String) {
        let images: Vec<(String, Vec<u8>)> = IMAGES.iter().map(|(a, png)| (a.file().to_owned(), png.to_vec())).collect();
        if let Err(e) = dock::images(&images) {
            log(LogLevel::Warn, &format!("dock-ghost: images refused: {e}"));
        }
        with(|l| {
            l.settings = serde_json::from_str(&json).unwrap_or_default();
            show(l);
        });
    }

    fn on_settings_changed(json: String) {
        with(|l| {
            l.settings = serde_json::from_str(&json).unwrap_or_default();
            show(l);
        });
    }

    fn on_event(event: AgentEvent) {
        let Some(surface) = event.surface.as_deref() else { return };
        with(|l| {
            if !event.verified && !l.settings.unverified {
                return;
            }
            let ev = Ev { surface, kind: &event.kind, cwd: event.cwd.as_deref(), summary: event.summary.as_deref(), subagent: event.agent_id.is_some() };
            if l.fleet.apply(&ev, now_ms()).is_some() {
                show(l);
            }
        });
    }

    fn palette_items(_mode: String, _query: String) -> Vec<PaletteItem> {
        Vec::new()
    }

    fn on_action(_action_id: String, _payload: String) {}

    fn row_menu(_surface: String) -> Vec<MenuItem> {
        Vec::new()
    }

    fn on_row_moved(_surface: String, _group: String, _before: Option<String>) {}

    fn on_theme_changed() {}

    fn on_tick(_now_ms: u64) {
        with(show);
    }

    fn on_exec(_job: u64, _outcome: Result<haunt::plugin::exec::Output, String>) {}

    fn on_away_changed(_away: bool, _away_ms: u64) {}

    fn on_hover_hold(_held: bool, _surface: String) {}

    fn on_fs_tree(_job: u64, _outcome: Result<haunt::plugin::fs_list::TreeResult, String>) {}
}

export!(Plugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tile_follows_the_pick() {
        let s = Settings::default();
        assert_eq!(tile(Face::Sleepy, &s), Tile { image: None, badge: None, blink: None }, "idle is Haunt's own icon");
        assert_eq!(tile(Face::Working, &s), Tile { image: Some("open"), badge: None, blink: Some("blink") });
        assert_eq!(tile(Face::Awake(2), &s), Tile { image: Some("needs"), badge: Some(2), blink: None });
        let off = Settings { badge: false, blink: false, ..s };
        assert_eq!((tile(Face::Working, &off).blink, tile(Face::Awake(2), &off).badge), (None, None));
    }

    #[test]
    fn manifest_asks_for_what_it_uses_and_defaults_match() {
        let m = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/haunt-plugin.toml")).unwrap();
        let t: toml::Table = toml::from_str(&m).unwrap();
        let caps = t["capabilities"].as_table().unwrap();
        assert_eq!(caps.keys().collect::<Vec<_>>(), ["dock", "events", "timer"]);
        let defaults: serde_json::Map<String, serde_json::Value> =
            t["settings"].as_table().unwrap().iter().map(|(k, v)| (k.clone(), serde_json::to_value(&v["default"]).unwrap())).collect();
        assert_eq!(serde_json::from_value::<Settings>(serde_json::Value::Object(defaults)).unwrap(), Settings::default());
    }
}
