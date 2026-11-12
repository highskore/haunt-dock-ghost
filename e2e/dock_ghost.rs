//! End to end against a real haunt core. Copy into haunt's `cli/tests/` and run
//! `cargo test -p haunt-cli --features plugins --test dock_ghost` (see README).
#![cfg(feature = "plugins")]

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use haunt_core::plugin_settings::PluginConfig;
use haunt_core::plugin_store::{PluginSet, Store, attach, take_requests_now};
use haunt_core::plugins::{HostRequest, PluginHost};
use haunt_core::{Core, SurfaceEnv};

const ID: &str = "dev.haunt.dock-ghost";

fn repo() -> PathBuf {
    std::env::var_os("HAUNT_PLUGINS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../haunt-plugins"))
        .join("haunt-dock-ghost")
}

fn send(sock: &mut UnixStream, env: &SurfaceEnv, nonce: bool, rest: &str) {
    let nonce = if nonce { format!(r#","nonce":"{}""#, env.nonce) } else { String::new() };
    let line = format!(r#"{{"v":1,"agent":"claude","surface_id":"{}"{nonce},"session_id":"s1","cwd":"/tmp/api",{rest}}}"#, env.surface_id);
    sock.write_all(format!("{line}\n").as_bytes()).unwrap();
}

/// The next Dock request, as (image id, badge, blink id).
fn next_show(set: &Mutex<PluginSet>) -> (Option<String>, Option<u32>, Option<String>) {
    let t = Instant::now();
    loop {
        let shows: Vec<_> = set.lock().unwrap().take_requests().into_iter().filter_map(|r| match r {
            HostRequest::DockShow { plugin_id, image, badge, blink, .. } if plugin_id == ID => {
                Some((image, badge, blink))
            }
            _ => None,
        }).collect();
        if let Some(s) = shows.into_iter().last() {
            return s;
        }
        assert!(t.elapsed() < Duration::from_secs(5), "timed out waiting for a Dock show");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn the_dock_ghost_wakes_with_the_agents() {
    let dir = repo();
    if !dir.join("haunt-plugin.toml").is_file() {
        eprintln!("skipped: haunt-dock-ghost not found; set HAUNT_PLUGINS_DIR");
        return;
    }
    let home = PathBuf::from(format!("/tmp/haunt-fp-dock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["build", "--release", "--target", "wasm32-wasip2"])
        .current_dir(&dir)
        .env_remove("CARGO_TARGET_DIR")
        .status()
        .unwrap();
    assert!(status.success());
    let o = Command::new(env!("CARGO_BIN_EXE_haunt")).args(["plugin", "install", dir.to_str().unwrap(), "--yes"]).env("HAUNT_HOME", &home).output().unwrap();
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));

    let core = Core::new(&home);
    core.start().unwrap();
    let (env, other) = (core.register_surface("/tmp").unwrap(), core.register_surface("/tmp").unwrap());
    let set = Store::open(&home).unwrap().load_all(&PluginHost::new().unwrap(), Some(&core.handle())).unwrap();
    assert!(set.errors().is_empty(), "{:?}", set.errors());
    let set = Arc::new(Mutex::new(set));
    attach(&core, &set);
    let s = |a: &str| Some(a.to_owned());
    let sent: Vec<_> = take_requests_now(&set).into_iter().filter_map(|r| match r {
        HostRequest::DockImages { plugin_id, images, dropped } if plugin_id == ID => Some((images.len(), dropped)),
        _ => None,
    }).collect();
    assert_eq!(sent, [(3, Vec::<String>::new())], "the three 512 px tiles, all decoded");

    let mut sock = UnixStream::connect(&env.socket_path).unwrap();
    send(&mut sock, &env, true, r#""event":"prompt_submit","query":"go""#);
    assert_eq!(next_show(&set), (s("open"), None, s("blink")), "working: open eyes that blink");
    send(&mut sock, &env, true, r#""event":"permission_seen","tool_name":"Bash""#);
    assert_eq!(next_show(&set), (s("needs"), Some(1), None), "needs you: orange eyes and the badge");

    set.lock().unwrap().set_config(&PluginConfig::parse(&format!("[plugins.\"{ID}\"]\nbadge = false\n")));
    assert_eq!(next_show(&set), (s("needs"), None, None));

    send(&mut sock, &other, false, r#""event":"permission_seen","tool_name":"Bash""#);
    send(&mut sock, &env, true, r#""event":"stop""#);
    assert_eq!(next_show(&set), (None, None, None), "asleep again, and the forged ask counted for nothing");

    drop(core);
    let _ = std::fs::remove_dir_all(&home);
}
