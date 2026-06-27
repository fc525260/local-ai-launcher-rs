use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub enum ServerEvent {
    Log(String),
}

pub struct ServerProcess {
    pub child: Child,
    pub rx: Receiver<ServerEvent>,
}

pub fn start_server(args: &[String], llama_cpp_dir: &Path) -> Result<ServerProcess, String> {
    if args.is_empty() {
        return Err("empty command".to_string());
    }
    let mut cmd = Command::new(llama_cpp_dir.join("llama-server.exe"));
    cmd.args(&args[1..])
        .current_dir(llama_cpp_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000);
    }

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (tx, rx) = mpsc::channel();

    if let Some(stdout) = stdout {
        let tx = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                let _ = tx.send(ServerEvent::Log(format!("[stdout] {line}")));
            }
        });
    }
    if let Some(stderr) = stderr {
        let tx = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                let _ = tx.send(ServerEvent::Log(format!("[stderr] {line}")));
            }
        });
    }

    Ok(ServerProcess { child, rx })
}

pub fn stop_process(child: &mut Child) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = child.kill();
    }
}
