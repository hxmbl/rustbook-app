//! Runs the book: copies the bundled source somewhere writable, starts
//! `mdbook serve` from the bundled binary, and waits for it to answer.

use std::{
    fs,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Manager};

use crate::paths;

/// Generous: the first build of the whole book takes about a second here, but
/// a slow disk and a cold page cache should not turn into a failed launch.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// A stable port keeps the book's origin (and therefore its saved theme choice
/// in `localStorage`) across launches. It is only a preference: if something
/// else already holds it, an ephemeral port is used instead.
const PREFERRED_PORT: u16 = 47823;

/// Set when the app is exiting, so a startup still in progress stops instead of
/// leaving an mdbook process behind.
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

pub fn shutdown() {
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
}

pub struct BookServer {
    child: Option<Child>,
    url: String,
    log: PathBuf,
}

impl BookServer {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn log_path(&self) -> &Path {
        &self.log
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for BookServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts `mdbook serve` on a free loopback port and blocks until it answers.
/// Call it from a background thread: it can take a couple of seconds.
pub fn start(app: &AppHandle) -> Result<BookServer, String> {
    let source = paths::book_source(app)?;
    let bins = paths::bin_dir(app)?;

    let cache = cache_dir(app);
    fs::create_dir_all(&cache).map_err(|err| {
        format!("could not create {}: {err}", cache.display())
    })?;
    let log = cache.join("mdbook.log");

    // Installed resources are read-only and mdbook writes its output next to the
    // book, so work from a copy. It is refreshed on every launch (about 11 MB,
    // a few dozen milliseconds) rather than cached by version, so a rebuilt app
    // never serves a stale book.
    let work = cache.join("book");
    stage(&source, &bins, &work)?;

    let port = free_port()?;
    let url = format!("http://127.0.0.1:{port}/");

    let stdout = fs::File::create(&log)
        .map_err(|err| format!("could not create {}: {err}", log.display()))?;
    let stderr = stdout
        .try_clone()
        .map_err(|err| format!("could not open {}: {err}", log.display()))?;

    // mdbook's own log is the only place a broken book.toml or a failing
    // preprocessor explains itself, so keep it in the cache directory.
    let mut command = Command::new(work.join("bin").join(exe("mdbook")));
    command
        .arg("serve")
        .arg(&work)
        .arg("--hostname")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--dest-dir")
        .arg(work.join("book"))
        .current_dir(&work)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    // If the app crashes or is killed, mdbook should not keep serving in the
    // background — and keep the port — with no window to show it in.
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            command.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                // The parent may have died between fork and prctl, in which case
                // the signal had nobody to notify.
                if libc::getppid() == 1 {
                    libc::_exit(1);
                }
                Ok(())
            });
        }
    }

    let mut child = command
        .spawn()
        .map_err(|err| format!("could not start mdbook: {err}"))?;

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    if let Err(err) = wait_until_ready(addr, &mut child, &log) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(err);
    }

    Ok(BookServer {
        child: Some(child),
        url,
        log,
    })
}

pub fn log_path(app: &AppHandle) -> PathBuf {
    cache_dir(app).join("mdbook.log")
}

fn cache_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_cache_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("rustbook"))
}

/// Copy the book into `work` and put the helper binaries in `work/bin`, which
/// is where the staged `book.toml` looks for the preprocessors.
fn stage(source: &Path, bins: &Path, work: &Path) -> Result<(), String> {
    if work.exists() {
        fs::remove_dir_all(work).map_err(|err| {
            format!("could not clear {}: {err}", work.display())
        })?;
    }
    copy_dir(source, work)?;

    let bin = work.join("bin");
    fs::create_dir_all(&bin)
        .map_err(|err| format!("could not create {}: {err}", bin.display()))?;
    for name in ["mdbook", "mdbook-trpl-note", "mdbook-trpl-listing", "mdbook-trpl-link-rewriter"] {
        let from = bins.join(exe(name));
        let to = bin.join(exe(name));
        fs::copy(&from, &to).map_err(|err| {
            format!(
                "could not copy {} to {}: {err}",
                from.display(),
                to.display()
            )
        })?;
        // Resources lose their mode bits when packaged, so restore them here.
        make_executable(&to)?;
    }
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to)
        .map_err(|err| format!("could not create {}: {err}", to.display()))?;
    let entries = fs::read_dir(from)
        .map_err(|err| format!("could not read {}: {err}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            format!("could not read {}: {err}", from.display())
        })?;
        let target = to.join(entry.file_name());
        let kind = entry.file_type().map_err(|err| {
            format!("could not inspect {}: {err}", entry.path().display())
        })?;
        if kind.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &target).map_err(|err| {
                format!(
                    "could not copy {} to {}: {err}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(
            |err| {
                format!("could not make {} executable: {err}", path.display())
            },
        )?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

fn free_port() -> Result<u16, String> {
    // Prefer the stable port so the book keeps the same origin — and with it
    // the theme choice mdbook stores in localStorage — between launches.
    if let Ok(listener) =
        TcpListener::bind((Ipv4Addr::LOCALHOST, PREFERRED_PORT))
    {
        drop(listener);
        return Ok(PREFERRED_PORT);
    }

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .map_err(|err| format!("could not reserve a port: {err}"))?;
    let port = listener
        .local_addr()
        .map_err(|err| format!("could not read the reserved port: {err}"))?
        .port();
    // mdbook binds it a moment later; the gap is small enough not to worry
    // about, and nothing else on the machine is racing for loopback ports.
    drop(listener);
    Ok(port)
}

fn wait_until_ready(
    addr: SocketAddr,
    child: &mut Child,
    log: &Path,
) -> Result<(), String> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        if SHUTTING_DOWN.load(Ordering::SeqCst) {
            return Err("the app is shutting down".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!(
                    "mdbook exited with {status} before the book was ready. See {}.",
                    log.display()
                ));
            }
            Ok(None) => {}
            Err(err) => {
                return Err(format!("could not check on mdbook: {err}"))
            }
        }
        if responds(addr) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "mdbook did not answer within {} seconds. See {}.",
                STARTUP_TIMEOUT.as_secs(),
                log.display()
            ));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// A plain HTTP request, rather than a TCP connect: mdbook binds the port only
/// after it has built the book, but answering 200 also proves the output is
/// actually being served.
fn responds(addr: SocketAddr) -> bool {
    let Ok(mut stream) =
        TcpStream::connect_timeout(&addr, Duration::from_millis(200))
    else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    if stream
        .write_all(b"GET / HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut head = [0u8; 12];
    if stream.read_exact(&mut head).is_err() {
        return false;
    }
    head.starts_with(b"HTTP/1.") && &head[9..12] == b"200"
}

/// Puts a readable failure on the loading page instead of leaving it spinning.
pub fn show_error(window: &tauri::WebviewWindow, message: &str, log: &Path) {
    let message =
        serde_json::to_string(message).unwrap_or_else(|_| "\"\"".into());
    let log = serde_json::to_string(&log.display().to_string())
        .unwrap_or_else(|_| "\"\"".into());
    let _ = window.set_title("The Rust Programming Language — could not start");
    let _ = window.eval(&format!(
        "(function show() {{
            if (window.showBookError) {{
                window.showBookError({message}, {log});
            }} else {{
                setTimeout(show, 100);
            }}
        }})();"
    ));
}
