use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use std::thread;
use std::{error, fmt};

use pgaf_sdk::context::{Context, ContextValue, PrimitiveContextValue};
use pgaf_sdk::pipeline::{Driver, PipelineStepType, PipelineStepTypeDriver};
use serde::Deserializer;
use serde::de::{IgnoredAny, MapAccess, Visitor};

/// Source for the child process's stdin.
pub enum StdinSource {
    /// Open the file at this path and redirect it into stdin.
    File(String),
    /// Pipe this already-evaluated string value directly into stdin.
    Value(String),
}

/// Destination for a captured output stream (stdout or stderr).
pub enum OutputSink {
    /// Write the output to the file at this path, truncating it if it exists.
    File(String),
    /// Capture the output as a UTF-8 string and store it in `ctx.run.extra` under this key.
    Key(String),
}

/// Arguments for the [`Cmd`] pipeline step. Constructed by the custom [`serde::Deserialize`]
/// implementation, which enforces mutual exclusivity between `stdin_file`/`stdin` and
/// `stdout_file`/`stdout` and `stderr_file`/`stderr` at deserialization time.
pub struct CmdArgs {
    /// Command string parsed with shell-style quoting (via `shlex`). The first token is the
    /// program; the rest are its arguments. No shell is invoked.
    pub cmd: String,
    /// Drop the context when the process exits with a non-zero status code. Default: `false`.
    pub fail_non_zero: bool,
    /// If set, store the process exit code (as an integer) into `ctx.run.extra` under this key.
    pub exit_code_key: Option<String>,
    /// Working directory for the child process. Defaults to the parent process's cwd.
    pub cwd: Option<String>,
    /// Whether to inherit the parent process's environment variables. Default: `true`.
    pub inherit_env: bool,
    /// Additional environment variables to set, specified as `env.KEY` keys in the args map.
    /// Merged on top of the inherited environment when `inherit_env` is `true`, or used as the
    /// sole environment when `inherit_env` is `false`.
    pub env: HashMap<String, String>,
    /// Stdin source. `stdin_file` and `stdin` are mutually exclusive.
    pub stdin: Option<StdinSource>,
    /// Stdout routing. `stdout_file` and `stdout` are mutually exclusive.
    pub stdout: Option<OutputSink>,
    /// Stderr routing. `stderr_file` and `stderr` are mutually exclusive.
    pub stderr: Option<OutputSink>,
}

impl<'de> serde::Deserialize<'de> for CmdArgs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(CmdArgsVisitor)
    }
}

struct CmdArgsVisitor;

impl<'de> Visitor<'de> for CmdArgsVisitor {
    type Value = CmdArgs;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a map of cmd step arguments")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<CmdArgs, A::Error> {
        let mut cmd: Option<String> = None;
        let mut fail_non_zero = false;
        let mut exit_code_key: Option<String> = None;
        let mut cwd: Option<String> = None;
        let mut inherit_env = true;
        let mut env: HashMap<String, String> = HashMap::new();
        let mut stdin_file: Option<String> = None;
        let mut stdin_val: Option<String> = None;
        let mut stdout_file: Option<String> = None;
        let mut stdout_key: Option<String> = None;
        let mut stderr_file: Option<String> = None;
        let mut stderr_key: Option<String> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "cmd" => cmd = Some(map.next_value()?),
                "fail_non_zero" => fail_non_zero = map.next_value()?,
                "exit_code_key" => exit_code_key = Some(map.next_value()?),
                "cwd" => cwd = Some(map.next_value()?),
                "inherit_env" => inherit_env = map.next_value()?,
                "stdin_file" => stdin_file = Some(map.next_value()?),
                "stdin" => stdin_val = Some(map.next_value()?),
                "stdout_file" => stdout_file = Some(map.next_value()?),
                "stdout" => stdout_key = Some(map.next_value()?),
                "stderr_file" => stderr_file = Some(map.next_value()?),
                "stderr" => stderr_key = Some(map.next_value()?),
                k if k.starts_with("env.") => {
                    let var = k.strip_prefix("env.").unwrap().to_string();
                    env.insert(var, map.next_value()?);
                }
                _ => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        let cmd = cmd.ok_or_else(|| serde::de::Error::missing_field("cmd"))?;

        let stdin = match (stdin_file, stdin_val) {
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "`stdin_file` and `stdin` are mutually exclusive",
                ));
            }
            (Some(f), None) => Some(StdinSource::File(f)),
            (None, Some(v)) => Some(StdinSource::Value(v)),
            (None, None) => None,
        };

        let stdout = match (stdout_file, stdout_key) {
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "`stdout_file` and `stdout` are mutually exclusive",
                ));
            }
            (Some(f), None) => Some(OutputSink::File(f)),
            (None, Some(k)) => Some(OutputSink::Key(k)),
            (None, None) => None,
        };

        let stderr = match (stderr_file, stderr_key) {
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "`stderr_file` and `stderr` are mutually exclusive",
                ));
            }
            (Some(f), None) => Some(OutputSink::File(f)),
            (None, Some(k)) => Some(OutputSink::Key(k)),
            (None, None) => None,
        };

        Ok(CmdArgs {
            cmd,
            fail_non_zero,
            exit_code_key,
            cwd,
            inherit_env,
            env,
            stdin,
            stdout,
            stderr,
        })
    }
}

#[derive(Debug)]
enum CmdError {
    EmptyCmd,
    ParseCmd,
    Io(std::io::Error),
}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CmdError::EmptyCmd => write!(f, "cmd string is empty"),
            CmdError::ParseCmd => write!(f, "failed to parse cmd string (unterminated quote?)"),
            CmdError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl error::Error for CmdError {}

impl From<std::io::Error> for CmdError {
    fn from(e: std::io::Error) -> Self {
        CmdError::Io(e)
    }
}

/// A pipeline step that runs an external command synchronously and routes its stdin, stdout,
/// and stderr to and from context values or files.
///
/// The `cmd` string is parsed with shell-style quoting rules (via [`shlex`]) but **does not
/// invoke a shell** — the first token is the program and the rest are its arguments.
///
/// Stdout and stderr are nulled by default; set `stdout`/`stderr` to capture them.
///
/// | Config key      | Type   | Required | Description |
/// |-----------------|--------|----------|-------------|
/// | `cmd`           | string | ✓        | Command string, shell-quoted, no shell invocation |
/// | `fail_non_zero` | bool   | ✗        | Drop context if exit code ≠ 0 (default: `false`) |
/// | `exit_code_key` | string | ✗        | Store exit code (int) into this context key |
/// | `cwd`           | string | ✗        | Working directory for the child process |
/// | `inherit_env`   | bool   | ✗        | Inherit the parent's environment (default: `true`) |
/// | `env.KEY`       | string | ✗        | Set env var `KEY`; merged on top of inherited env |
/// | `stdin_file`    | string | ✗        | Read stdin from this path *(exclusive with `stdin`)* |
/// | `stdin`         | string | ✗        | Pipe this value into stdin *(exclusive with `stdin_file`)* |
/// | `stdout_file`   | string | ✗        | Write stdout to this path *(exclusive with `stdout`)* |
/// | `stdout`        | string | ✗        | Capture stdout into this context key *(exclusive with `stdout_file`)* |
/// | `stderr_file`   | string | ✗        | Write stderr to this path *(exclusive with `stderr`)* |
/// | `stderr`        | string | ✗        | Capture stderr into this context key *(exclusive with `stderr_file`)* |
pub struct Cmd;

impl PipelineStepType<CmdArgs> for Cmd {
    fn invoke(
        stream: Box<dyn Iterator<Item = (CmdArgs, Context)>>,
    ) -> Box<dyn Iterator<Item = Context>> {
        Box::new(stream.filter_map(|(args, mut ctx)| {
            let unit_id = ctx.unit.id.clone();
            match run_cmd(args, &mut ctx) {
                Ok(true) => Some(ctx),
                Ok(false) => None,
                Err(e) => {
                    eprintln!("cmd: error at unit {unit_id}: {e}");
                    None
                }
            }
        }))
    }
}

fn run_cmd(args: CmdArgs, ctx: &mut Context) -> Result<bool, CmdError> {
    let parts: Vec<String> = shlex::split(&args.cmd)
        .ok_or(CmdError::ParseCmd)?
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return Err(CmdError::EmptyCmd);
    }

    let mut command = Command::new(&parts[0]);
    command.args(&parts[1..]);

    if !args.inherit_env {
        command.env_clear();
    }
    for (k, v) in args.env {
        command.env(k, v);
    }
    if let Some(cwd) = args.cwd {
        command.current_dir(cwd);
    }

    let stdin_data: Option<String> = match args.stdin {
        None => {
            command.stdin(Stdio::null());
            None
        }
        Some(StdinSource::File(path)) => {
            command.stdin(Stdio::from(std::fs::File::open(path)?));
            None
        }
        Some(StdinSource::Value(val)) => {
            command.stdin(Stdio::piped());
            Some(val)
        }
    };

    let stdout_key: Option<String> = match args.stdout {
        None => {
            command.stdout(Stdio::null());
            None
        }
        Some(OutputSink::File(path)) => {
            command.stdout(Stdio::from(std::fs::File::create(path)?));
            None
        }
        Some(OutputSink::Key(key)) => {
            command.stdout(Stdio::piped());
            Some(key)
        }
    };

    let stderr_key: Option<String> = match args.stderr {
        None => {
            command.stderr(Stdio::null());
            None
        }
        Some(OutputSink::File(path)) => {
            command.stderr(Stdio::from(std::fs::File::create(path)?));
            None
        }
        Some(OutputSink::Key(key)) => {
            command.stderr(Stdio::piped());
            Some(key)
        }
    };

    let mut child = command.spawn()?;

    // Drive all streams concurrently to prevent pipe-buffer deadlocks.
    let stdin_thread = stdin_data.and_then(|data| {
        child.stdin.take().map(|mut stdin| {
            thread::spawn(move || {
                let _ = stdin.write_all(data.as_bytes());
            })
        })
    });

    let stdout_thread = child.stdout.take().map(|mut out| {
        thread::spawn(move || {
            let mut buf = String::new();
            let _ = out.read_to_string(&mut buf);
            buf
        })
    });

    let stderr_thread = child.stderr.take().map(|mut err| {
        thread::spawn(move || {
            let mut buf = String::new();
            let _ = err.read_to_string(&mut buf);
            buf
        })
    });

    let status = child.wait()?;

    if let Some(t) = stdin_thread {
        let _ = t.join();
    }
    let stdout_data = stdout_thread.map(|t| t.join().unwrap_or_default());
    let stderr_data = stderr_thread.map(|t| t.join().unwrap_or_default());

    if let Some(key) = args.exit_code_key {
        let code = status.code().unwrap_or(-1) as i64;
        ctx.data
            .insert(key, ContextValue::Prim(PrimitiveContextValue::Int(code)));
    }
    if let (Some(key), Some(data)) = (stdout_key, stdout_data) {
        ctx.data
            .insert(key, ContextValue::Prim(PrimitiveContextValue::String(data)));
    }
    if let (Some(key), Some(data)) = (stderr_key, stderr_data) {
        ctx.data
            .insert(key, ContextValue::Prim(PrimitiveContextValue::String(data)));
    }

    if args.fail_non_zero && !status.success() {
        return Ok(false);
    }

    Ok(true)
}

pub static CMD_DRIVER: LazyLock<Driver> =
    LazyLock::new(|| PipelineStepTypeDriver::<Cmd, CmdArgs>::default().coerce_to_dynamic());

#[cfg(all(test, unix))] // I don't even bother making this run on Windows.
mod tests {
    use super::*;
    use crate::pipeline::make_ctx;
    use pgaf_sdk::context::{ContextValue, PrimitiveContextValue};
    use pgaf_sdk::pipeline::PipelineStepTypeArgs;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn map_args(
        pairs: impl IntoIterator<Item = (&'static str, ContextValue)>,
    ) -> PipelineStepTypeArgs {
        PipelineStepTypeArgs::Map(
            pairs
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect::<HashMap<_, _>>(),
        )
    }

    fn str(s: &str) -> ContextValue {
        ContextValue::Prim(PrimitiveContextValue::String(s.into()))
    }

    fn bool(b: bool) -> ContextValue {
        ContextValue::Prim(PrimitiveContextValue::Bool(b))
    }

    #[test]
    fn passes_through_on_success() {
        let args = map_args([("cmd", str("true")), ("fail_non_zero", bool(true))]);

        let result: Vec<_> = CMD_DRIVER
            .invoke(Arc::new(args), Box::new(vec![make_ctx(1)].into_iter()))
            .collect();

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn drops_on_nonzero_when_fail_non_zero() {
        let args = map_args([("cmd", str("false")), ("fail_non_zero", bool(true))]);

        let result: Vec<_> = CMD_DRIVER
            .invoke(
                Arc::new(args),
                Box::new(vec![make_ctx(1), make_ctx(2)].into_iter()),
            )
            .collect();

        assert!(result.is_empty());
    }

    #[test]
    fn keeps_on_nonzero_by_default() {
        let args = map_args([("cmd", str("false"))]);

        let result: Vec<_> = CMD_DRIVER
            .invoke(Arc::new(args), Box::new(vec![make_ctx(1)].into_iter()))
            .collect();

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn exit_code_stored_in_context() {
        let args = map_args([("cmd", str("false")), ("exit_code_key", str("code"))]);

        let ctx = CMD_DRIVER
            .invoke(Arc::new(args), Box::new(vec![make_ctx(1)].into_iter()))
            .next()
            .unwrap();

        assert_eq!(
            ctx.data.get("code"),
            Some(&ContextValue::Prim(PrimitiveContextValue::Int(1)))
        );
    }

    #[test]
    fn stdout_captured_to_key() {
        let args = map_args([("cmd", str("echo hello")), ("stdout", str("out"))]);

        let ctx = CMD_DRIVER
            .invoke(Arc::new(args), Box::new(vec![make_ctx(1)].into_iter()))
            .next()
            .unwrap();

        assert_eq!(
            ctx.data.get("out"),
            Some(&ContextValue::Prim(PrimitiveContextValue::String(
                "hello\n".into()
            )))
        );
    }

    #[test]
    fn stdin_piped_to_stdout_via_cat() {
        let args = map_args([
            ("cmd", str("cat")),
            ("stdin", str("world")),
            ("stdout", str("out")),
        ]);

        let ctx = CMD_DRIVER
            .invoke(Arc::new(args), Box::new(vec![make_ctx(1)].into_iter()))
            .next()
            .unwrap();

        assert_eq!(
            ctx.data.get("out"),
            Some(&ContextValue::Prim(PrimitiveContextValue::String(
                "world".into()
            )))
        );
    }

    #[test]
    fn stderr_captured_to_key() {
        let args = map_args([
            ("cmd", str("ls /this/path/does/not/exist")),
            ("stderr", str("err")),
        ]);

        let ctx = CMD_DRIVER
            .invoke(Arc::new(args), Box::new(vec![make_ctx(1)].into_iter()))
            .next()
            .unwrap();

        match ctx.data.get("err") {
            Some(ContextValue::Prim(PrimitiveContextValue::String(s))) => {
                assert!(!s.is_empty());
            }
            other => panic!("expected non-empty string, got {other:?}"),
        }
    }

    #[test]
    fn mutual_exclusivity_stdin_drops_context() {
        let args = map_args([
            ("cmd", str("true")),
            ("stdin", str("a")),
            ("stdin_file", str("/dev/null")),
        ]);

        let result: Vec<_> = CMD_DRIVER
            .invoke(Arc::new(args), Box::new(vec![make_ctx(1)].into_iter()))
            .collect();

        assert!(result.is_empty());
    }

    #[test]
    fn quoted_args_parsed_correctly() {
        let args = map_args([("cmd", str("echo 'hello world'")), ("stdout", str("out"))]);

        let ctx = CMD_DRIVER
            .invoke(Arc::new(args), Box::new(vec![make_ctx(1)].into_iter()))
            .next()
            .unwrap();

        assert_eq!(
            ctx.data.get("out"),
            Some(&ContextValue::Prim(PrimitiveContextValue::String(
                "hello world\n".into()
            )))
        );
    }

    #[test]
    fn env_var_is_set() {
        let args = map_args([
            ("cmd", str("printenv PGAF_TEST_VAR")),
            ("env.PGAF_TEST_VAR", str("sentinel")),
            ("stdout", str("out")),
        ]);

        let ctx = CMD_DRIVER
            .invoke(Arc::new(args), Box::new(vec![make_ctx(1)].into_iter()))
            .next()
            .unwrap();

        assert_eq!(
            ctx.data.get("out"),
            Some(&ContextValue::Prim(PrimitiveContextValue::String(
                "sentinel\n".into()
            )))
        );
    }

    #[test]
    fn cleared_env_does_not_inherit_path() {
        // With inherit_env=false and no env vars set, PATH is absent so common
        // commands won't resolve. We verify by checking the exit code of `env`
        // (the /usr/bin/env binary) with an absolute path — it succeeds — but
        // a naked `printenv PATH` would fail to find PATH in the output.
        let args = map_args([
            ("cmd", str("printenv PATH")),
            ("inherit_env", bool(false)),
            ("stdout", str("out")),
            ("exit_code_key", str("code")),
        ]);

        let ctx = CMD_DRIVER
            .invoke(Arc::new(args), Box::new(vec![make_ctx(1)].into_iter()))
            .next()
            .unwrap();

        // printenv exits 1 when the variable is not set
        assert_eq!(
            ctx.data.get("code"),
            Some(&ContextValue::Prim(PrimitiveContextValue::Int(1)))
        );
    }

    #[test]
    fn cwd_changes_working_directory() {
        let args = map_args([
            ("cmd", str("pwd")),
            ("cwd", str("/tmp")),
            ("stdout", str("out")),
        ]);

        let ctx = CMD_DRIVER
            .invoke(Arc::new(args), Box::new(vec![make_ctx(1)].into_iter()))
            .next()
            .unwrap();

        assert_eq!(
            ctx.data.get("out"),
            Some(&ContextValue::Prim(PrimitiveContextValue::String(
                "/tmp\n".into()
            )))
        );
    }
}
