//! Minimal Linux child-subreaper supervisor spike for CapyDeck research.
//!
//! This is a test tool, not production code.
//!
//! It sets `PR_SET_CHILD_SUBREAPER`, spawns one command, tracks the main child
//! and all discovered descendants, reaps orphaned descendants, and emits JSON
//! lifecycle events. It never sends signals to children on its own (except the
//! explicitly test-only `--stop-after` option).
//!
//! Build (offline, no target dir in the repo):
//!   CARGO_TARGET_DIR=/tmp/capydeck-supervisor-target \
//!     cargo build --release --manifest-path docs/spikes/session-supervisor/Cargo.toml --offline
//!
//! Usage:
//!   .../session-supervisor --wait cgroup --events /tmp/events.jsonl -- <command> [args...]
//!
//! See docs/spikes/README.md and docs/SESSION-SUPERVISOR-SPIKE-RESULTS.md.

use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::raw::{c_int, c_long};
use std::os::unix::io::RawFd;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PR_SET_CHILD_SUBREAPER: c_int = 36;
const PR_GET_CHILD_SUBREAPER: c_int = 37;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WaitMode {
    Main,
    Cgroup,
}

struct Options {
    wait: WaitMode,
    timeout: Option<Duration>,
    stop_after: Option<Duration>,
    stop_signal: c_int,
    events: Option<PathBuf>,
    child_stdio_null: bool,
    cwd: Option<PathBuf>,
    new_session: bool,
    pidfd: bool,
    settle: Duration,
    discover: Duration,
    command: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            wait: WaitMode::Cgroup,
            timeout: None,
            stop_after: None,
            stop_signal: libc::SIGTERM,
            events: None,
            child_stdio_null: false,
            cwd: None,
            new_session: false,
            pidfd: false,
            settle: Duration::from_millis(300),
            discover: Duration::from_millis(100),
            command: Vec::new(),
        }
    }
}

fn usage() -> ! {
    eprintln!(
        "usage: session-supervisor [options] -- <command> [args...]\n\
         \n\
         options:\n\
           --wait main|cgroup       exit when main child or whole known tree is gone (default: cgroup)\n\
           --timeout SECS           overall test timeout\n\
           --stop-after SECS        test-only: signal the main child after N seconds\n\
           --stop-signal TERM|KILL|INT  signal for --stop-after (default: TERM)\n\
           --events PATH            write JSON lines to PATH (default: stdout)\n\
           --child-stdio inherit|null  child stdio (default: inherit)\n\
           --cwd PATH               child working directory\n\
           --new-session            call setsid() in the child\n\
           --pidfd                  open pidfds for tracked processes\n\
           --settle-ms N            settle time before declaring cgroup empty (default: 300)\n\
           --discover-ms N          /proc discovery interval (default: 100)\n\
           --help"
    );
    std::process::exit(2);
}

fn parse_signal(s: &str) -> c_int {
    match s.to_ascii_uppercase().as_str() {
        "TERM" | "SIGTERM" => libc::SIGTERM,
        "KILL" | "SIGKILL" => libc::SIGKILL,
        "INT" | "SIGINT" => libc::SIGINT,
        "HUP" | "SIGHUP" => libc::SIGHUP,
        "QUIT" | "SIGQUIT" => libc::SIGQUIT,
        _ => {
            eprintln!("unknown signal: {s}");
            std::process::exit(2);
        }
    }
}

fn parse_options() -> Options {
    let mut opts = Options::default();
    let mut args = env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => usage(),
            "--" => {
                opts.command.extend(args);
                break;
            }
            "--wait" => {
                let value = args.next().unwrap_or_else(|| usage());
                opts.wait = match value.as_str() {
                    "main" => WaitMode::Main,
                    "cgroup" => WaitMode::Cgroup,
                    _ => usage(),
                };
            }
            "--timeout" => {
                let value = args.next().unwrap_or_else(|| usage());
                opts.timeout = Some(Duration::from_secs(
                    value.parse().unwrap_or_else(|_| usage()),
                ));
            }
            "--stop-after" => {
                let value = args.next().unwrap_or_else(|| usage());
                opts.stop_after = Some(Duration::from_secs(
                    value.parse().unwrap_or_else(|_| usage()),
                ));
            }
            "--stop-signal" => {
                let value = args.next().unwrap_or_else(|| usage());
                opts.stop_signal = parse_signal(&value);
            }
            "--events" => {
                opts.events = Some(PathBuf::from(args.next().unwrap_or_else(|| usage())));
            }
            "--child-stdio" => {
                let value = args.next().unwrap_or_else(|| usage());
                opts.child_stdio_null = match value.as_str() {
                    "inherit" => false,
                    "null" => true,
                    _ => usage(),
                };
            }
            "--cwd" => {
                opts.cwd = Some(PathBuf::from(args.next().unwrap_or_else(|| usage())));
            }
            "--new-session" => opts.new_session = true,
            "--pidfd" => opts.pidfd = true,
            "--settle-ms" => {
                let value = args.next().unwrap_or_else(|| usage());
                opts.settle = Duration::from_millis(value.parse().unwrap_or_else(|_| usage()));
            }
            "--discover-ms" => {
                let value = args.next().unwrap_or_else(|| usage());
                opts.discover = Duration::from_millis(value.parse().unwrap_or_else(|_| usage()));
            }
            other => {
                eprintln!("unknown option: {other}");
                usage();
            }
        }
    }
    if opts.command.is_empty() {
        usage();
    }
    opts
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

struct EventSink {
    out: Box<dyn Write>,
    t0: Instant,
    boot_id: String,
}

impl EventSink {
    fn new(path: Option<&PathBuf>) -> io::Result<Self> {
        let out: Box<dyn Write> = match path {
            Some(path) => Box::new(OpenOptions::new().create(true).append(true).open(path)?),
            None => Box::new(io::stdout()),
        };
        Ok(Self {
            out,
            t0: Instant::now(),
            boot_id: fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .unwrap_or_default()
                .trim()
                .to_owned(),
        })
    }

    fn emit(&mut self, event: &str, fields: &[(&str, String)]) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let t_unix = format!("{}.{:03}", now.as_secs(), now.subsec_millis());
        let t_ms = self.t0.elapsed().as_millis().to_string();
        let mut line = format!(
            "{{\"t_unix\":\"{}\",\"t_ms\":\"{}\",\"event\":\"{}\",\"boot_id\":\"{}\"",
            t_unix,
            t_ms,
            json_escape(event),
            json_escape(&self.boot_id)
        );
        for (key, value) in fields {
            line.push_str(&format!(
                ",\"{}\":\"{}\"",
                json_escape(key),
                json_escape(value)
            ));
        }
        line.push('}');
        let _ = writeln!(self.out, "{line}");
        let _ = self.out.flush();
    }
}

fn read_text(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn read_stat(pid: i32) -> Option<(i32, u64, String)> {
    let stat = read_text(&format!("/proc/{pid}/stat"))?;
    let rparen = stat.rfind(')')?;
    let rest = stat.get(rparen + 2..)?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    if fields.len() < 20 {
        return None;
    }
    let state = fields[0].to_owned();
    let ppid = fields[1].parse().ok()?;
    let starttime = fields[19].parse().ok()?;
    Some((ppid, starttime, state))
}

fn read_comm(pid: i32) -> String {
    read_text(&format!("/proc/{pid}/comm"))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn read_argv0(pid: i32) -> String {
    let Some(raw) = fs::read(format!("/proc/{pid}/cmdline")).ok() else {
        return String::new();
    };
    let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
    let mut s = String::from_utf8_lossy(&raw[..end]).into_owned();
    if s.len() > 256 {
        s.truncate(256);
    }
    s
}

fn read_cgroup(pid: i32) -> String {
    read_text(&format!("/proc/{pid}/cgroup"))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn read_children(pid: i32) -> Vec<i32> {
    let path = format!("/proc/{pid}/task/{pid}/children");
    read_text(&path)
        .map(|s| {
            s.split_whitespace()
                .filter_map(|v| v.parse::<i32>().ok())
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Clone)]
struct ProcInfo {
    pid: i32,
    ppid: i32,
    starttime: u64,
    state: String,
    comm: String,
    argv0: String,
    cgroup: String,
    source: &'static str,
}

fn proc_info(pid: i32, ppid_hint: i32, source: &'static str) -> Option<ProcInfo> {
    let (ppid, starttime, state) = read_stat(pid)?;
    Some(ProcInfo {
        pid,
        ppid: if ppid > 0 { ppid } else { ppid_hint },
        starttime,
        state,
        comm: read_comm(pid),
        argv0: read_argv0(pid),
        cgroup: read_cgroup(pid),
        source,
    })
}

fn describe_status(status: c_int) -> (Option<c_int>, Option<c_int>) {
    if libc::WIFEXITED(status) {
        (Some(libc::WEXITSTATUS(status)), None)
    } else if libc::WIFSIGNALED(status) {
        (None, Some(libc::WTERMSIG(status)))
    } else {
        (None, None)
    }
}

fn block_signals() -> libc::sigset_t {
    unsafe {
        let mut old: libc::sigset_t = std::mem::zeroed();
        let mut mask: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut old);
        libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut old);
        libc::sigemptyset(&mut mask);
        libc::sigaddset(&mut mask, libc::SIGCHLD);
        libc::sigaddset(&mut mask, libc::SIGTERM);
        libc::sigaddset(&mut mask, libc::SIGINT);
        libc::pthread_sigmask(libc::SIG_BLOCK, &mask, std::ptr::null_mut());
        old
    }
}

fn open_signalfd() -> c_int {
    unsafe {
        let mut mask: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut mask);
        libc::sigaddset(&mut mask, libc::SIGCHLD);
        libc::sigaddset(&mut mask, libc::SIGTERM);
        libc::sigaddset(&mut mask, libc::SIGINT);
        libc::signalfd(-1, &mask, libc::SFD_CLOEXEC | libc::SFD_NONBLOCK)
    }
}

fn pidfd_open(pid: i32) -> Option<RawFd> {
    unsafe {
        let fd = libc::syscall(libc::SYS_pidfd_open, pid as c_long, 0 as c_long) as c_long;
        if fd >= 0 {
            Some(fd as RawFd)
        } else {
            None
        }
    }
}

fn pidfd_send_signal(fd: RawFd, sig: c_int) -> bool {
    unsafe {
        let rc = libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd as c_long,
            sig as c_long,
            std::ptr::null::<libc::siginfo_t>(),
            0 as c_long,
        ) as c_long;
        rc == 0
    }
}

fn main() {
    let opts = parse_options();
    let mut sink = EventSink::new(opts.events.as_ref()).unwrap_or_else(|err| {
        eprintln!("cannot open event sink: {err}");
        std::process::exit(2);
    });

    let self_pid = std::process::id() as i32;
    let reaper_rc = unsafe { libc::prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) };
    let mut is_subreaper: c_int = -1;
    unsafe {
        libc::prctl(PR_GET_CHILD_SUBREAPER, &mut is_subreaper, 0, 0, 0);
    }
    sink.emit(
        "supervisor_start",
        &[
            ("pid", self_pid.to_string()),
            ("wait", format!("{:?}", opts.wait).to_ascii_lowercase()),
            ("subreaper_rc", reaper_rc.to_string()),
            ("is_subreaper", is_subreaper.to_string()),
            ("pidfd", opts.pidfd.to_string()),
            ("new_session", opts.new_session.to_string()),
        ],
    );

    let old_mask = block_signals();
    let sfd = open_signalfd();
    if sfd < 0 {
        sink.emit(
            "signalfd_failed",
            &[("errno", io::Error::last_os_error().to_string())],
        );
        std::process::exit(2);
    }

    let mut cmd = Command::new(&opts.command[0]);
    cmd.args(&opts.command[1..]);
    if opts.child_stdio_null {
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
    }
    if let Some(cwd) = &opts.cwd {
        cmd.current_dir(cwd);
    }
    let new_session = opts.new_session;
    unsafe {
        cmd.pre_exec(move || {
            libc::pthread_sigmask(libc::SIG_SETMASK, &old_mask, std::ptr::null_mut());
            if new_session {
                libc::setsid();
            }
            Ok(())
        });
    }

    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            sink.emit(
                "spawn_failed",
                &[("argv", opts.command.join(" ")), ("error", err.to_string())],
            );
            std::process::exit(2);
        }
    };
    let main_pid = child.id() as i32;
    let mut tracked: HashMap<i32, ProcInfo> = HashMap::new();
    let mut pidfds: HashMap<i32, RawFd> = HashMap::new();
    let mut main_alive = true;
    let mut main_exited_at: Option<Instant> = None;
    let mut stop_sent = false;
    let mut exit_reason = "cgroup_empty";
    let mut exit_code = 0;
    let start = Instant::now();

    if let Some(info) = proc_info(main_pid, self_pid, "spawn") {
        sink.emit(
            "spawn",
            &[
                ("pid", info.pid.to_string()),
                ("ppid", info.ppid.to_string()),
                ("starttime", info.starttime.to_string()),
                ("comm", info.comm.clone()),
                ("state", info.state.clone()),
                ("argv0", info.argv0.clone()),
                ("cgroup", info.cgroup.clone()),
                ("argv", opts.command.join(" ")),
            ],
        );
        if opts.pidfd {
            if let Some(fd) = pidfd_open(info.pid) {
                pidfds.insert(info.pid, fd);
            }
        }
        tracked.insert(main_pid, info);
    } else {
        sink.emit("spawn_identity_failed", &[("pid", main_pid.to_string())]);
    }

    let mut last_scan = Instant::now() - opts.discover;
    let mut last_cgroups: HashMap<i32, String> = tracked
        .iter()
        .map(|(pid, info)| (*pid, info.cgroup.clone()))
        .collect();

    loop {
        let elapsed = start.elapsed();
        if let Some(timeout) = opts.timeout {
            if elapsed >= timeout {
                sink.emit(
                    "timeout",
                    &[("elapsed_ms", elapsed.as_millis().to_string())],
                );
                exit_reason = "timeout";
                exit_code = 124;
                break;
            }
        }

        if !stop_sent {
            if let Some(stop_after) = opts.stop_after {
                if elapsed >= stop_after && main_alive {
                    let sent = if let Some(fd) = pidfds.get(&main_pid) {
                        pidfd_send_signal(*fd, opts.stop_signal)
                    } else {
                        unsafe { libc::kill(main_pid, opts.stop_signal) == 0 }
                    };
                    sink.emit(
                        "test_signal",
                        &[
                            ("pid", main_pid.to_string()),
                            ("signal", opts.stop_signal.to_string()),
                            ("pidfd", pidfds.contains_key(&main_pid).to_string()),
                            ("ok", sent.to_string()),
                        ],
                    );
                    stop_sent = true;
                }
            }
        }

        let mut pfd = libc::pollfd {
            fd: sfd,
            events: libc::POLLIN,
            revents: 0,
        };
        let wait_ms = 50;
        let poll_rc = unsafe { libc::poll(&mut pfd, 1, wait_ms) };
        if poll_rc > 0 && (pfd.revents & libc::POLLIN) != 0 {
            loop {
                let mut info: libc::signalfd_siginfo = unsafe { std::mem::zeroed() };
                let n = unsafe {
                    libc::read(
                        sfd,
                        &mut info as *mut _ as *mut libc::c_void,
                        std::mem::size_of::<libc::signalfd_siginfo>(),
                    )
                };
                if n != std::mem::size_of::<libc::signalfd_siginfo>() as isize {
                    break;
                }
                let signo = info.ssi_signo as c_int;
                match signo {
                    libc::SIGCHLD => loop {
                        let mut status: c_int = 0;
                        let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
                        if pid <= 0 {
                            break;
                        }
                        let (code, signal) = describe_status(status);
                        let info_opt = tracked.remove(&(pid as i32));
                        if let Some(info) = &info_opt {
                            sink.emit(
                                "exit",
                                &[
                                    ("pid", info.pid.to_string()),
                                    ("starttime", info.starttime.to_string()),
                                    ("comm", info.comm.clone()),
                                    ("source", info.source.to_string()),
                                    ("code", code.map(|v| v.to_string()).unwrap_or_default()),
                                    ("signal", signal.map(|v| v.to_string()).unwrap_or_default()),
                                ],
                            );
                            if let Some(fd) = pidfds.remove(&info.pid) {
                                unsafe { libc::close(fd) };
                            }
                            if info.pid == main_pid {
                                main_alive = false;
                                main_exited_at = Some(Instant::now());
                                sink.emit(
                                    "main_exit",
                                    &[
                                        ("pid", main_pid.to_string()),
                                        ("code", code.map(|v| v.to_string()).unwrap_or_default()),
                                        (
                                            "signal",
                                            signal.map(|v| v.to_string()).unwrap_or_default(),
                                        ),
                                    ],
                                );
                            }
                        } else {
                            sink.emit("exit_untracked", &[("pid", pid.to_string())]);
                        }
                    },
                    libc::SIGTERM | libc::SIGINT => {
                        sink.emit("supervisor_signal", &[("signal", signo.to_string())]);
                        exit_reason = "signal";
                        exit_code = 128 + signo;
                        break;
                    }
                    other => {
                        sink.emit("other_signal", &[("signal", other.to_string())]);
                    }
                }
                if exit_reason == "signal" {
                    break;
                }
            }
        }

        if exit_reason == "signal" {
            break;
        }

        if last_scan.elapsed() >= opts.discover {
            last_scan = Instant::now();
            let mut to_scan: Vec<i32> = vec![self_pid];
            to_scan.extend(tracked.keys().copied());
            for parent in to_scan {
                for child_pid in read_children(parent) {
                    if tracked.contains_key(&child_pid) {
                        continue;
                    }
                    let source = if parent == self_pid {
                        "adopted"
                    } else {
                        "child"
                    };
                    if let Some(info) = proc_info(child_pid, parent, source) {
                        if opts.pidfd {
                            if let Some(fd) = pidfd_open(info.pid) {
                                pidfds.insert(info.pid, fd);
                            }
                        }
                        sink.emit(
                            if source == "adopted" {
                                "adopted"
                            } else {
                                "discovered"
                            },
                            &[
                                ("pid", info.pid.to_string()),
                                ("ppid", info.ppid.to_string()),
                                ("starttime", info.starttime.to_string()),
                                ("comm", info.comm.clone()),
                                ("state", info.state.clone()),
                                ("argv0", info.argv0.clone()),
                                ("cgroup", info.cgroup.clone()),
                            ],
                        );
                        last_cgroups.insert(info.pid, info.cgroup.clone());
                        tracked.insert(info.pid, info);
                    }
                }
            }

            // Detect cgroup changes and disappearances for non-child descendants.
            let pids: Vec<i32> = tracked.keys().copied().collect();
            for pid in pids {
                if let Some(info) = tracked.get(&pid) {
                    if let Some(cgroup) = read_text(&format!("/proc/{pid}/cgroup")) {
                        let cgroup = cgroup.trim().to_owned();
                        if last_cgroups.get(&pid) != Some(&cgroup) {
                            sink.emit(
                                "cgroup_changed",
                                &[
                                    ("pid", pid.to_string()),
                                    ("from", last_cgroups.get(&pid).cloned().unwrap_or_default()),
                                    ("to", cgroup.clone()),
                                ],
                            );
                            last_cgroups.insert(pid, cgroup);
                        }
                    } else if fs::metadata(format!("/proc/{pid}")).is_err() {
                        let info = info.clone();
                        tracked.remove(&pid);
                        if let Some(fd) = pidfds.remove(&pid) {
                            unsafe { libc::close(fd) };
                        }
                        sink.emit(
                            "disappeared",
                            &[
                                ("pid", info.pid.to_string()),
                                ("starttime", info.starttime.to_string()),
                                ("comm", info.comm.clone()),
                                ("source", info.source.to_string()),
                            ],
                        );
                        if info.pid == main_pid {
                            main_alive = false;
                            main_exited_at.get_or_insert_with(Instant::now);
                            sink.emit("main_exit", &[("pid", main_pid.to_string())]);
                        }
                    }
                }
            }
        }

        if opts.wait == WaitMode::Main && main_exited_at.is_some() {
            let remaining = tracked.len();
            if remaining > 0 {
                sink.emit("children_remaining", &[("count", remaining.to_string())]);
            }
            exit_reason = "main_exit";
            break;
        }

        if main_exited_at.is_some() {
            let settle_done = main_exited_at
                .map(|t| t.elapsed() >= opts.settle)
                .unwrap_or(false);
            if settle_done && tracked.is_empty() {
                exit_reason = "cgroup_empty";
                break;
            }
        }
    }

    let remaining = tracked.len();
    sink.emit(
        "supervisor_exit",
        &[
            ("reason", exit_reason.to_string()),
            ("remaining", remaining.to_string()),
            ("elapsed_ms", start.elapsed().as_millis().to_string()),
        ],
    );
    for (_, fd) in pidfds {
        unsafe { libc::close(fd) };
    }
    unsafe { libc::close(sfd) };
    std::process::exit(exit_code);
}
