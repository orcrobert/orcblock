use std::sync::{Arc, Mutex};

use super::*;

type Respond = dyn Fn(&str, &[&str]) -> Option<String> + Send + Sync;

/// Answers from a function and records each call as one line.
#[derive(Clone)]
struct FakeRunner {
    respond: Arc<Respond>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl FakeRunner {
    fn new(respond: impl Fn(&str, &[&str]) -> Option<String> + Send + Sync + 'static) -> Self {
        Self {
            respond: Arc::new(respond),
            calls: Arc::default(),
        }
    }

    fn count(&self, program: &str) -> usize {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|line| line.starts_with(program))
            .count()
    }
}

impl Runner for FakeRunner {
    fn run(&self, program: &str, args: &[&str]) -> Option<String> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(format!("{program} {}", args.join(" ")));
        (self.respond)(program, args)
    }
}

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake";

/// A temp dir holding fake app bundles and the icon cache.
struct Disk {
    dir: tempfile::TempDir,
}

impl Disk {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("temp dir"),
        }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// Creates `<rel>/Contents/Info.plist` and an icon in Resources.
    fn bundle(&self, rel: &str) -> String {
        let bundle = self.path(rel);
        fs::create_dir_all(bundle.join("Contents/Resources")).expect("creates bundle");
        fs::write(bundle.join("Contents/Info.plist"), "").expect("writes plist");
        fs::write(bundle.join("Contents/Resources/AppIcon.icns"), "icns").expect("writes icon");
        bundle.to_string_lossy().into_owned()
    }
}

/// Info.plist answers by bundle path: `Some(json)` converts, `None` fails
/// like a plist holding dates.
fn plists(entries: Vec<(String, Option<Value>)>) -> impl Fn(&str, &[&str]) -> Option<String> {
    let entries: HashMap<String, Option<Value>> = entries.into_iter().collect();
    move |program, args| {
        if program != PLUTIL {
            return None;
        }
        let plist = args.last()?;
        let bundle = plist.strip_suffix("/Contents/Info.plist")?;
        let info = entries.get(bundle)?;
        match (args.first().copied(), info) {
            (Some("-convert"), Some(info)) => Some(info.to_string()),
            (Some("-extract"), None) if args[1] == "CFBundleIdentifier" => {
                Some("com.example.Dated\n".into())
            }
            _ => None,
        }
    }
}

fn apps_in(disk: &Disk, runner: FakeRunner) -> Apps<FakeRunner> {
    Apps::new(
        runner,
        vec![disk.path("Applications"), disk.path("Missing")],
        disk.path("cache"),
    )
}

#[test]
fn scans_folders_for_apps_with_valid_ids() {
    let disk = Disk::new();
    let slack = disk.bundle("Applications/Slack.app");
    let zoom = disk.bundle("Applications/zoom.us.app");
    let dated = disk.bundle("Applications/Dated.app");
    let bad = disk.bundle("Applications/Bad.app");
    let nameless = disk.bundle("Applications/Nameless Thing.app");
    fs::create_dir_all(disk.path("Applications/NotAnApp")).expect("creates dir");
    let runner = FakeRunner::new(plists(vec![
        (
            slack.clone(),
            Some(
                serde_json::json!({"CFBundleIdentifier": "com.tinyspeck.slackmacgap", "CFBundleName": "Slack"}),
            ),
        ),
        (
            zoom,
            Some(
                serde_json::json!({"CFBundleIdentifier": "us.zoom.xos", "CFBundleDisplayName": "zoom.us", "CFBundleName": "zoom"}),
            ),
        ),
        (dated, None),
        (
            bad,
            Some(serde_json::json!({"CFBundleIdentifier": "../etc"})),
        ),
        (
            nameless,
            Some(
                serde_json::json!({"CFBundleIdentifier": "com.example.Nameless", "CFBundleName": " "}),
            ),
        ),
    ]));
    let apps = apps_in(&disk, runner.clone());
    let found = apps.installed();
    let summary: Vec<(&str, &str)> = found
        .iter()
        .map(|app| (app.name.as_str(), app.bundle_id.as_str()))
        .collect();
    assert_eq!(
        summary,
        [
            ("Dated", "com.example.dated"),
            ("Nameless Thing", "com.example.nameless"),
            ("Slack", "com.tinyspeck.slackmacgap"),
            ("zoom.us", "us.zoom.xos"),
        ]
    );
    assert_eq!(found[2].path, slack);

    let calls = runner.count(PLUTIL);
    apps.installed();
    assert_eq!(runner.count(PLUTIL), calls, "scanned once per session");
}

#[test]
fn reads_a_chosen_or_dropped_app_and_remembers_it() {
    let disk = Disk::new();
    let chess = disk.bundle("Elsewhere/Chess.app");
    let runner = FakeRunner::new(plists(vec![(
        chess.clone(),
        Some(serde_json::json!({"CFBundleIdentifier": "com.apple.Chess", "CFBundleName": "Chess"})),
    )]));
    let apps = apps_in(&disk, runner);
    let cases = [
        ("relative", "Chess.app"),
        ("not an app", "/etc/passwd"),
        ("missing bundle", "/nowhere/Gone.app"),
    ];
    for (case, path) in cases {
        assert_eq!(apps.at_path(Path::new(path)), None, "{case}");
    }
    let app = apps.at_path(Path::new(&chess)).expect("reads Chess");
    assert_eq!(app.bundle_id, "com.apple.chess");
    assert!(apps.installed().contains(&app), "listed from now on");
}

#[test]
fn picks_with_a_constant_script() {
    let disk = Disk::new();
    let chess = disk.bundle("Elsewhere/Chess.app");
    let chosen = format!("{chess}/\n");
    let info = plists(vec![(
        chess,
        Some(serde_json::json!({"CFBundleIdentifier": "com.apple.Chess"})),
    )]);
    let runner = FakeRunner::new(move |program, args| {
        if program == OSASCRIPT {
            assert_eq!(args, ["-e", CHOOSE_APP]);
            return Some(chosen.clone());
        }
        info(program, args)
    });
    let apps = apps_in(&disk, runner);
    assert_eq!(
        apps.pick().map(|app| app.bundle_id),
        Some("com.apple.chess".into())
    );

    let cancelled = apps_in(&disk, FakeRunner::new(|_, _| None));
    assert_eq!(cancelled.pick(), None);
}

#[test]
fn lists_running_apps_by_bundle_id() {
    let disk = Disk::new();
    let slack = disk.bundle("Applications/Slack.app");
    let copy = disk.bundle("Desktop/Slack copy.app");
    let table = format!(
        "/sbin/launchd\n{slack}/Contents/MacOS/Slack\n{slack}/Contents/Frameworks/Slack Helper.app/Contents/MacOS/Slack Helper\n{copy}/Contents/MacOS/Slack\n-zsh\n"
    );
    let info = plists(vec![
        (
            slack,
            Some(serde_json::json!({"CFBundleIdentifier": "com.tinyspeck.slackmacgap"})),
        ),
        (
            copy,
            Some(serde_json::json!({"CFBundleIdentifier": "com.tinyspeck.slackmacgap"})),
        ),
    ]);
    let runner = FakeRunner::new(move |program, args| {
        if program == ENV {
            assert_eq!(args, PS_ARGS);
            return Some(table.clone());
        }
        info(program, args)
    });
    let apps = apps_in(&disk, runner);
    assert_eq!(apps.running(), ["com.tinyspeck.slackmacgap"]);
}

#[test]
fn converts_and_caches_icons() {
    let disk = Disk::new();
    let slack = disk.bundle("Applications/Slack.app");
    let info = plists(vec![(
        slack,
        Some(
            serde_json::json!({"CFBundleIdentifier": "com.tinyspeck.slackmacgap", "CFBundleIconFile": "AppIcon"}),
        ),
    )]);
    let runner = FakeRunner::new(move |program, args| {
        if program == SIPS {
            assert!(
                args[5].ends_with("Contents/Resources/AppIcon.icns"),
                "{args:?}"
            );
            fs::write(args[7], PNG).expect("writes png");
            return Some(String::new());
        }
        info(program, args)
    });
    let apps = apps_in(&disk, runner.clone());
    let url = apps.icon("com.tinyspeck.SlackMacGap").expect("icon");
    assert!(url.starts_with("data:image/png;base64,"), "{url}");
    assert_eq!(apps.icon("com.tinyspeck.slackmacgap"), Some(url));
    assert_eq!(runner.count(SIPS), 1, "the second call is cached");
    assert_eq!(apps.icon("com.example.unknown"), None);
    assert_eq!(apps.icon("../etc"), None);
}

#[test]
fn ignores_icon_names_with_paths() {
    let disk = Disk::new();
    let evil = disk.bundle("Applications/Evil.app");
    let runner = FakeRunner::new(plists(vec![(
        evil,
        Some(
            serde_json::json!({"CFBundleIdentifier": "com.example.evil", "CFBundleIconFile": "../../../../etc/secret"}),
        ),
    )]));
    let apps = apps_in(&disk, runner.clone());
    assert_eq!(apps.icon("com.example.evil"), None);
    assert_eq!(runner.count(SIPS), 0);
}

#[test]
fn checks_app_paths() {
    let cases = [
        ("app", "/Applications/Slack.app", true),
        ("upper case", "/Applications/GAME.APP", true),
        ("relative", "Slack.app", false),
        ("bare suffix", "/Applications/.app", false),
        ("inside a bundle", "/Applications/Slack.app/Contents", false),
        ("other suffix", "/Applications/Slack.application", false),
    ];
    for (case, path, want) in cases {
        assert_eq!(is_app_path(Path::new(path)), want, "{case}");
    }
}
