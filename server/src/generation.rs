//! Generic case-generation engine.
//!
//! The engine knows nothing about any specific question (mechanism, not
//! policy). A question folder ships its own `generator` and `solver` commands
//! — a Python script, a compiled Rust binary, or anything executable. The
//! engine simply runs them:
//!
//! * generator: invoked as `<cmd...> <seed>` with `CASE_SEED=<seed>` in the
//!   environment; writes one test **input** to stdout.
//! * solver: reads that input on **stdin**; writes the expected **output** to
//!   stdout.
//!
//! Both run with their working directory set to the question folder, so
//! relative paths (`./generator`, `data/table.csv`) resolve naturally. Both
//! must be deterministic for a given seed/input.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context};

/// How long a single generator or solver invocation may run before it is
/// killed. Authored content is trusted, so this only guards against mistakes.
const TIMEOUT: Duration = Duration::from_secs(20);

fn program_path(dir: &Path, program: &str) -> PathBuf {
    // A program that names a path (contains a separator or a leading `.`) is
    // resolved against the question folder; a bare name (`python3`) is left
    // for the OS to find on PATH.
    if program.starts_with('.') || program.contains('/') {
        dir.join(program)
    } else {
        PathBuf::from(program)
    }
}

/// Captured result of running a child process.
struct Captured {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Wait for `child` to finish, concurrently draining stdout/stderr (and, when
/// present, writing `input` to stdin) so a large stream can never deadlock on
/// a full pipe buffer. Kills the child if it exceeds the time limit.
fn wait_with_timeout(mut child: Child, input: Option<String>) -> anyhow::Result<Captured> {
    // Drain each stream on its own thread; otherwise a child that writes more
    // than the OS pipe buffer (~64 KiB) blocks forever waiting for us to read.
    let mut stdout = child.stdout.take().context("child stdout unavailable")?;
    let mut stderr = child.stderr.take().context("child stderr unavailable")?;
    let out_reader = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout.read_to_end(&mut buffer);
        buffer
    });
    let err_reader = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stderr.read_to_end(&mut buffer);
        buffer
    });
    let stdin_writer = child.stdin.take().map(|mut stdin| {
        let payload = input.unwrap_or_default();
        thread::spawn(move || {
            let _ = stdin.write_all(payload.as_bytes());
            // Dropping stdin here closes the pipe so the child sees EOF.
        })
    });

    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().context("polling child process")? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            // Reader threads unblock once the pipes close on kill.
            let _ = out_reader.join();
            let _ = err_reader.join();
            if let Some(writer) = stdin_writer {
                let _ = writer.join();
            }
            bail!("process exceeded the {}s time limit", TIMEOUT.as_secs());
        }
        thread::sleep(Duration::from_millis(5));
    };

    if let Some(writer) = stdin_writer {
        let _ = writer.join();
    }
    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    Ok(Captured {
        status,
        stdout,
        stderr,
    })
}

/// Run a question's generator with the given seed, returning the test input.
pub fn run_generator(dir: &Path, cmd: &[String], seed: u64) -> anyhow::Result<String> {
    let (program, args) = cmd.split_first().context("generator command is empty")?;
    let child = Command::new(program_path(dir, program))
        .args(args)
        .arg(seed.to_string())
        .env("CASE_SEED", seed.to_string())
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("could not start generator {cmd:?} in {}", dir.display()))?;

    let output = wait_with_timeout(child, None)?;
    if !output.status.success() {
        bail!(
            "generator {cmd:?} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("generator output was not UTF-8")
}

/// Run a question's solver over `input`, returning the expected output.
pub fn run_solver(dir: &Path, cmd: &[String], input: &str) -> anyhow::Result<String> {
    let (program, args) = cmd.split_first().context("solver command is empty")?;
    let child = Command::new(program_path(dir, program))
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("could not start solver {cmd:?} in {}", dir.display()))?;

    let output = wait_with_timeout(child, Some(input.to_string()))?;
    if !output.status.success() {
        bail!(
            "solver {cmd:?} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("solver output was not UTF-8")
}

/// A stable per-(team, question, index) seed. Regenerating after a restart
/// yields identical cases because generation is deterministic by seed.
pub fn case_seed(team_id: &str, question_id: &str, index: usize) -> u64 {
    fnv1a(&format!("{team_id}:{question_id}:{index}"))
}

fn fnv1a(text: &str) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325_u64;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}
