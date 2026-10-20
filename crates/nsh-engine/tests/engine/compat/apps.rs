//! Apps built by earlier checkouts run on this one: `api/apps/` keeps what
//! some checkout's `nsh build --release` wrote for some apps
//! (`<app>/bundle.js`; packages always carry their source, which runs when
//! their bytecode is not this runtime's) and a script of what a user does
//! and sees (`<app>/script.txt`), one step per line (with the assets it
//! imports, `<app>/assets/`):
//!
//! - `viewport <width> <height>`: before anything else, the screen's size
//!   (390 × 844 by default);
//! - `platform <os> [desktop]`: before anything else (with `viewport`), the
//!   platform the app runs on: `ios` (the default), `android`, `macos`,
//!   `windows`, `linux` or `web`; `desktop` makes a browser a desktop one.
//!   It decides which of `ios={{…}}`, `android={{…}}`, `desktop={{…}}`
//!   (the desktop OSes and desktop browsers) and `web={{…}}` apply;
//! - `expect <text>`: a text on screen reads `<text>` (waits for it);
//! - `absent <text>`: no text on screen reads `<text>` (after the app is
//!   quiet);
//! - `tap <selector>`: presses the first view on screen `id:<testID>`,
//!   `label:<accessibility label>` or `text:<text>` names, or its nearest
//!   pressable ancestor (as `nsh trace --tap`);
//! - `wait <ms>`: lets timers and animations run;
//! - `frame <selector> <x> <y> <width> <height>`: the first view on screen
//!   the selector names (itself, pressable or not) is laid out at that
//!   frame, in points relative to its parent (waits for it);
//! - `prop <selector> <name> <value>`: the first view on screen the
//!   selector names has host prop `<name>` (its JS name; a platform's props
//!   with or without their `ios.` / `android.` / `desktop.` / `web.`
//!   prefix) at `<value>` as the platform receives it, written as
//!   compat/values.rs says (waits for it);
//! - `noprop <selector> <name>`: the view has no such prop, or the value a
//!   view of its kind has with none set (after the app is quiet);
//! - `style <selector> <key> <value>`, `nostyle <selector> <key>`: the
//!   same for a style key, as mounted (compat/values.rs: paint keys, and
//!   the text keys of texts and inputs; layout keys show in `frame`s). A
//!   selector `span:<text>` names the first text that contains `<text>`,
//!   and its text keys are those of the run where `<text>` starts (nested
//!   `<Text>`s);
//! - `listens <selector> <event>`: the platform reports event `<event>`
//!   (`onPress`, ...) of the view, so its handler is set (waits for it);
//! - `log <text>`: the app printed a `console` line (any level) containing
//!   `<text>` (waits for it);
//! - `covers <item>`: no step, a claim: the script checks what `<item>` (a
//!   part of the API as `api/uncovered.txt` names it) does (compat/coverage.rs
//!   verifies it).
//!
//! A selector is one word (`id:<testID>`, `label:<label>`, `text:<text>`);
//! the steps after `frame` also take the part after `:` quoted, for one
//! with spaces: `text:"Taps: 0"`. Any error the app reports fails it. `NSH_API_DUMP=1` prints the texts
//! on screen after each step, for writing scripts.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, OnceLock},
    thread::ThreadId,
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use nsh_core::{
    ApproxTextEngine, EventKind, EventPayload, MountBatch, MountItem, NodeId, Rect, Size,
    platform::{Environment, Idiom, Os},
};
use nsh_engine::{Assets, Bundle, Engine, EngineConfig, EngineControl, MountSink};

use super::values::{self, Mounted};

/// The surface's root view.
const ROOT: NodeId = 1;

/// How long an `expect` waits for its text.
const EXPECT_WAIT: Duration = Duration::from_secs(5);

/// How long a `tap` waits at most for the app to be quiet first (a ticking
/// app never is).
const TAP_WAIT: Duration = Duration::from_millis(1000);

/// How long the app must send nothing to count as quiet.
const QUIET: Duration = Duration::from_millis(300);

struct Sink {
    batches: Sender<MountBatch>,
    errors: Arc<Mutex<Vec<String>>>,
    /// The engine's thread, which runs the app's JS: what it logs is the
    /// app's.
    thread: Arc<OnceLock<ThreadId>>,
}

impl MountSink for Sink {
    fn submit(&self, batch: MountBatch) {
        self.thread.get_or_init(|| std::thread::current().id());
        let _ = self.batches.send(batch);
    }

    fn report_error(&self, message: &str) {
        self.errors.lock().unwrap().push(message.to_owned());
    }
}

/// `asset:/<path>` from the app's `assets/`.
struct AppAssets(std::path::PathBuf);

impl Assets for AppAssets {
    fn read(&self, uri: &str) -> Result<Vec<u8>, String> {
        let path = uri
            .strip_prefix("asset:/")
            .ok_or_else(|| format!("not an asset URI: {uri}"))?;
        std::fs::read(self.0.join(path)).map_err(|e| format!("{uri}: {e}"))
    }
}

/// The native view tree as the mount items built it.
#[derive(Default)]
struct Views {
    nodes: HashMap<NodeId, View>,
    children: HashMap<NodeId, Vec<NodeId>>,
}

#[derive(Default)]
struct View {
    parent: Option<NodeId>,
    pressable: bool,
    test_id: Option<Arc<str>>,
    label: Option<Arc<str>>,
    text: Option<String>,
    frame: Option<Rect>,
    /// What the platform got for it, as `prop` and `style` read it.
    mounted: Mounted,
}

impl Views {
    fn apply(&mut self, batch: &MountBatch) {
        for item in &batch.items {
            match item {
                MountItem::Create { id, .. } => {
                    self.nodes.insert(*id, View::default());
                }
                MountItem::Insert {
                    parent,
                    child,
                    before,
                } => {
                    self.detach(*child);
                    let siblings = self.children.entry(*parent).or_default();
                    let at = before
                        .and_then(|b| siblings.iter().position(|s| *s == b))
                        .unwrap_or(siblings.len());
                    siblings.insert(at, *child);
                    self.nodes.entry(*child).or_default().parent = Some(*parent);
                }
                MountItem::Remove { id } => self.detach(*id),
                MountItem::Delete { id } => {
                    self.detach(*id);
                    self.nodes.remove(id);
                    self.children.remove(id);
                }
                MountItem::Events { id, events } => {
                    self.nodes.entry(*id).or_default().pressable =
                        events.contains(EventKind::Press);
                }
                MountItem::ViewProps { id, props } => {
                    let view = self.nodes.entry(*id).or_default();
                    view.test_id = props.test_id.clone();
                    view.label = props.accessibility.label.clone();
                }
                MountItem::Text { id, text, .. } => {
                    self.nodes.entry(*id).or_default().text = Some(text.text.clone());
                }
                MountItem::Frame { id, frame, .. } => {
                    self.nodes.entry(*id).or_default().frame = Some(*frame);
                }
                _ => {}
            }
            if let Some(id) = Mounted::target(item) {
                self.nodes.entry(id).or_default().mounted.apply(item);
            }
        }
    }

    fn detach(&mut self, id: NodeId) {
        if let Some(parent) = self.nodes.get_mut(&id).and_then(|v| v.parent.take())
            && let Some(siblings) = self.children.get_mut(&parent)
        {
            siblings.retain(|s| *s != id);
        }
    }

    /// The views on screen (attached below the root), in tree order.
    fn attached(&self) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![ROOT];
        while let Some(id) = stack.pop() {
            if id != ROOT {
                out.push(id);
            }
            if let Some(children) = self.children.get(&id) {
                stack.extend(children.iter().rev());
            }
        }
        out
    }

    fn texts(&self) -> Vec<String> {
        self.attached()
            .iter()
            .filter_map(|id| self.nodes.get(id)?.text.as_deref())
            .map(|t| t.trim().to_owned())
            .collect()
    }

    fn shows(&self, text: &str) -> bool {
        self.texts().iter().any(|t| t == text)
    }

    /// The first view on screen `selector` names, or its nearest pressable
    /// ancestor.
    fn find(&self, selector: &str) -> Option<NodeId> {
        self.find_view(selector)
            .and_then(|id| self.pressable_from(id))
    }

    /// The first view on screen `selector` names.
    fn find_view(&self, selector: &str) -> Option<NodeId> {
        let is = |value: &Option<Arc<str>>, want: &str| value.as_deref() == Some(want);
        let test: Box<dyn Fn(&View) -> bool> = if let Some(want) = selector.strip_prefix("id:") {
            Box::new(move |v| is(&v.test_id, want))
        } else if let Some(want) = selector.strip_prefix("label:") {
            Box::new(move |v| is(&v.label, want))
        } else if let Some(want) = selector.strip_prefix("text:") {
            Box::new(move |v| v.text.as_deref().map(str::trim) == Some(want))
        } else if let Some(want) = selector.strip_prefix("span:") {
            Box::new(move |v| v.text.as_deref().is_some_and(|t| t.contains(want)))
        } else {
            panic!("selector `{selector}`: expected id:, label:, text: or span:")
        };
        self.attached()
            .into_iter()
            .find(|id| self.nodes.get(id).is_some_and(&test))
    }

    /// The mounted state of the first view on screen `selector` names, and
    /// for `span:<text>`, the UTF-16 offset in its text where `<text>`
    /// starts.
    fn mounted(&self, selector: &str) -> Option<(&Mounted, Option<u32>)> {
        let view = self.nodes.get(&self.find_view(selector)?)?;
        let at = selector.strip_prefix("span:").and_then(|want| {
            let text = view.text.as_deref()?;
            let byte = text.find(want)?;
            Some(text[..byte].encode_utf16().count() as u32)
        });
        Some((&view.mounted, at))
    }

    fn pressable_from(&self, mut id: NodeId) -> Option<NodeId> {
        loop {
            let view = self.nodes.get(&id)?;
            if view.pressable {
                return Some(id);
            }
            id = view.parent?;
        }
    }
}

struct Run {
    _engine: Engine,
    control: EngineControl,
    rx: Receiver<MountBatch>,
    errors: Arc<Mutex<Vec<String>>>,
    thread: Arc<OnceLock<ThreadId>>,
    views: Views,
}

impl Run {
    fn start(bundle: Bundle, viewport: Size, environment: Environment, assets: &Path) -> Run {
        crate::logs::capture();
        let (tx, rx) = crossbeam_channel::unbounded();
        let errors = Arc::new(Mutex::new(Vec::new()));
        let thread = Arc::new(OnceLock::new());
        let config = EngineConfig {
            app: nsh_engine::App::Js(bundle),
            viewport,
            trace: false,
            network: None,
            assets: Some(Arc::new(AppAssets(assets.to_path_buf()))),
            environment,
            header_heights: (44.0, 96.0),
            limits: nsh_engine::Limits::default(),
            ui_sets: vec![nsh_ui_mobile::registry()],
            runtime: None,
            data_dir: None,
            storage: None,
        };
        let sink = Sink {
            batches: tx,
            errors: errors.clone(),
            thread: thread.clone(),
        };
        let engine = Engine::start(config, || ApproxTextEngine, sink).unwrap();
        Run {
            control: engine.control(),
            _engine: engine,
            rx,
            errors,
            thread,
            views: Views::default(),
        }
    }

    /// Whether the app printed a line containing `text`.
    fn printed(&self, text: &str) -> bool {
        self.thread
            .get()
            .is_some_and(|t| crate::logs::printed_by(*t).iter().any(|l| l.contains(text)))
    }

    /// Lets the app run until `done` holds or `limit` ends.
    fn poll(&mut self, limit: Duration, done: impl Fn(&Run) -> bool) -> bool {
        let end = Instant::now() + limit;
        loop {
            if done(self) {
                return true;
            }
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            match self.rx.recv_timeout(left.min(Duration::from_millis(20))) {
                Ok(batch) => self.views.apply(&batch),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => panic!("the engine stopped"),
            }
        }
    }

    /// Applies the batches that arrive until `done` holds or `limit` ends.
    fn until(&mut self, limit: Duration, done: impl Fn(&Views) -> bool) -> bool {
        let end = Instant::now() + limit;
        while !done(&self.views) {
            let left = end.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(batch) => self.views.apply(&batch),
                Err(RecvTimeoutError::Timeout) => return false,
                Err(RecvTimeoutError::Disconnected) => panic!("the engine stopped"),
            }
        }
        true
    }

    /// Applies batches until none came for [`QUIET`] (or `limit` ends).
    fn settle(&mut self, limit: Duration) {
        let end = Instant::now() + limit;
        loop {
            let left = end.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(QUIET.min(left)) {
                Ok(batch) => self.views.apply(&batch),
                Err(_) => return,
            }
        }
    }

    fn errors(&self) -> Vec<String> {
        self.errors.lock().unwrap().clone()
    }
}

/// Runs the app in `dir` (`bundle.js`, `script.txt`); the first failed step
/// is the error.
pub(crate) fn run_app(dir: &Path) -> Result<(), String> {
    let code = std::fs::read_to_string(dir.join("bundle.js")).map_err(|e| e.to_string())?;
    let script = std::fs::read_to_string(dir.join("script.txt")).map_err(|e| e.to_string())?;
    run(code, &script, &dir.join("assets")).map_err(|f| f.message)
}

/// Why a script failed.
pub(crate) struct Failure {
    /// The step that failed, when one of its checks did (not an error the
    /// app reported, nor a malformed script).
    pub(crate) check: Option<String>,
    /// Whether the app reported errors.
    pub(crate) errors: bool,
    pub(crate) message: String,
}

impl From<String> for Failure {
    fn from(message: String) -> Failure {
        Failure {
            check: None,
            errors: false,
            message,
        }
    }
}

impl From<&str> for Failure {
    fn from(message: &str) -> Failure {
        message.to_owned().into()
    }
}

/// Runs the bundle `code` with its `assets` through `script`.
pub(crate) fn run(code: String, script: &str, assets: &Path) -> Result<(), Failure> {
    let dump = std::env::var_os("NSH_API_DUMP").is_some();
    let mut lines = script
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .peekable();
    let mut viewport = Size::new(390.0, 844.0);
    let mut environment = Environment::default();
    while let Some(line) = lines.peek() {
        if let Some(size) = line.strip_prefix("viewport ") {
            let (w, h) = size.split_once(' ').ok_or("viewport <width> <height>")?;
            viewport = Size::new(w.parse().unwrap(), h.parse().unwrap());
        } else if let Some(platform) = line.strip_prefix("platform ") {
            let mut words = platform.split_whitespace();
            environment.os = match words.next() {
                Some("ios") => Os::Ios,
                Some("android") => Os::Android,
                Some("macos") => Os::MacOs,
                Some("windows") => Os::Windows,
                Some("linux") => Os::Linux,
                Some("web") => Os::Web,
                _ => {
                    return Err(format!(
                        "`{line}`: platform ios|android|macos|windows|linux|web [desktop]"
                    )
                    .into());
                }
            };
            let desktop = match words.next() {
                None => environment.os.is_desktop(),
                Some("desktop") => true,
                Some(other) => return Err(format!("`{line}`: `{other}`, not `desktop`").into()),
            };
            if desktop {
                environment.idiom = Idiom::Desktop;
            }
        } else {
            break;
        }
        lines.next();
    }
    let mut run = Run::start(Bundle::source(code), viewport, environment, assets);
    if !run.until(EXPECT_WAIT, |v| !v.attached().is_empty()) {
        return Err(Failure {
            check: None,
            errors: !run.errors().is_empty(),
            message: format!("the app rendered nothing; errors: {:?}", run.errors()),
        });
    }
    if dump {
        run.settle(EXPECT_WAIT);
        println!("(start)\n  {:?}", run.views.texts());
    }
    let mut shown = run.views.texts();
    for (n, line) in lines.enumerate() {
        let (step, arg) = line.split_once(' ').unwrap_or((line, ""));
        let failed = |what: String, run: &Run| {
            let errors = run.errors();
            Err(Failure {
                check: (what != "the app reported errors").then(|| line.to_owned()),
                errors: !errors.is_empty(),
                message: format!(
                    "step {} `{line}`: {what}\non screen: {:?}\nerrors: {errors:?}",
                    n + 1,
                    run.views.texts(),
                ),
            })
        };
        match step {
            "expect" => {
                if !run.until(EXPECT_WAIT, |v| v.shows(arg)) {
                    return failed("not shown".into(), &run);
                }
            }
            "absent" => {
                run.settle(EXPECT_WAIT);
                if run.views.shows(arg) {
                    return failed("still shown".into(), &run);
                }
            }
            "tap" => {
                run.settle(TAP_WAIT);
                let Some(id) = run.views.find(arg) else {
                    return failed("nothing to press".into(), &run);
                };
                run.control
                    .dispatch_event(id, EventKind::Press, EventPayload::None);
            }
            "wait" => {
                let ms: u64 = arg.parse().map_err(|_| format!("wait `{arg}`"))?;
                run.until(Duration::from_millis(ms), |_| false);
            }
            "frame" => {
                let mut parts = arg.split(' ');
                let selector = parts.next().unwrap_or_default();
                let want: Vec<f32> = parts.filter_map(|p| p.parse().ok()).collect();
                let [x, y, w, h] = want[..] else {
                    return Err(
                        format!("frame `{arg}`: <selector> <x> <y> <width> <height>").into(),
                    );
                };
                let at = |v: &Views| v.find_view(selector).and_then(|id| v.nodes.get(&id)?.frame);
                let fits = |v: &Views| {
                    at(v).is_some_and(|f| {
                        [f.x - x, f.y - y, f.width - w, f.height - h]
                            .iter()
                            .all(|d| d.abs() < 0.5)
                    })
                };
                if !run.until(EXPECT_WAIT, fits) {
                    let what = match at(&run.views) {
                        Some(f) => format!("laid out at {} {} {} {}", f.x, f.y, f.width, f.height),
                        None => "no such view".to_owned(),
                    };
                    return failed(what, &run);
                }
            }
            "prop" | "style" => {
                let (selector, rest) = selector(arg);
                let (name, want) = rest
                    .split_once(' ')
                    .ok_or_else(|| format!("`{line}`: {step} <selector> <name> <value>"))?;
                let (key, want) = (values::key(name), want.trim());
                let all = |v: &Views| {
                    let (m, at) = v.mounted(&selector)?;
                    Some(if step == "prop" {
                        m.props()
                    } else {
                        m.styles(at)
                    })
                };
                let read = |v: &Views| all(v)?.remove(key);
                if !run.until(EXPECT_WAIT, |v| read(v).as_deref() == Some(want)) {
                    let what = match all(&run.views) {
                        None => "no such view".to_owned(),
                        Some(all) => {
                            // What the view has, for writing the step.
                            let has: Vec<String> =
                                all.iter().map(|(k, v)| format!("{k} {v}")).collect();
                            let got = match all.get(key) {
                                None => format!("no {step} `{key}`"),
                                Some(got) => format!("`{key}` is `{got}`"),
                            };
                            format!("{got}\nthe view's {step}s:\n  {}", has.join("\n  "))
                        }
                    };
                    return failed(what, &run);
                }
            }
            "noprop" | "nostyle" => {
                let (selector, name) = selector(arg);
                let key = values::key(name.trim());
                run.settle(EXPECT_WAIT);
                let Some((m, at)) = run.views.mounted(&selector) else {
                    return failed("no such view".into(), &run);
                };
                let (got, unset) = if step == "noprop" {
                    (m.props().remove(key), m.blank().props().remove(key))
                } else {
                    (m.styles(at).remove(key), m.blank().styles(at).remove(key))
                };
                if let Some(got) = got.filter(|g| Some(g) != unset.as_ref()) {
                    return failed(format!("`{key}` is `{got}`"), &run);
                }
            }
            "listens" => {
                let (selector, event) = selector(arg);
                let event = event.trim();
                let kind = EventKind::from_prop(event)
                    .ok_or_else(|| format!("`{line}`: `{event}` is no event"))?;
                let listens =
                    |v: &Views| v.mounted(&selector).is_some_and(|(m, _)| m.listens(kind));
                if !run.until(EXPECT_WAIT, listens) {
                    return failed(format!("`{event}` not reported"), &run);
                }
            }
            "log" => {
                if !run.poll(EXPECT_WAIT, |r| r.printed(arg)) {
                    return failed("not printed".into(), &run);
                }
            }
            "covers" => continue,
            other => return Err(format!("unknown step `{other}`").into()),
        }
        if dump {
            run.settle(EXPECT_WAIT);
            let now = run.views.texts();
            let added: Vec<_> = now.iter().filter(|t| !shown.contains(t)).collect();
            let gone: Vec<_> = shown.iter().filter(|t| !now.contains(t)).collect();
            println!("{line}\n  + {added:?}\n  - {gone:?}");
            shown = now;
        }
        if !run.errors().is_empty() {
            return failed("the app reported errors".into(), &run);
        }
    }
    run.settle(EXPECT_WAIT);
    match run.errors() {
        errors if errors.is_empty() => Ok(()),
        errors => Err(Failure {
            check: None,
            errors: true,
            message: format!("the app reported errors: {errors:?}"),
        }),
    }
}

/// Splits a step's argument into its selector and the rest: the selector
/// is one word, or a prefix and a quoted value (`text:"Taps: 0"`).
fn selector(arg: &str) -> (String, &str) {
    if let Some((prefix, quoted)) = arg.split_once(":\"")
        && !prefix.contains(' ')
        && let Some((value, rest)) = quoted.split_once('"')
    {
        return (format!("{prefix}:{value}"), rest.trim_start());
    }
    match arg.split_once(' ') {
        Some((selector, rest)) => (selector.to_owned(), rest),
        None => (arg.to_owned(), ""),
    }
}
