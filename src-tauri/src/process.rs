use std::time::Duration;
use tokio::process::Command;

pub async fn run(program: &str, args: &[String], timeout: Duration) -> Result<String, String> {
    let mut command = Command::new(program);
    command.args(args).kill_on_drop(true);
    // Polling helpers must not create a console or activate Windows Terminal.
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let output = tokio::time::timeout(timeout, command.output())
        .await
        .map_err(|_| format!("{program} timed out"))?
        .map_err(|e| format!("Could not start {program}: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(format!(
            "{program} failed: {}",
            if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            }
        ));
    }
    Ok(stdout)
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn background_commands_have_no_console_and_still_capture_output() {
        let script = r#"Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class ConsoleProbe { [DllImport("kernel32.dll")] public static extern IntPtr GetConsoleWindow(); }'; [ConsoleProbe]::GetConsoleWindow().ToInt64()"#;
        let args = ["-NoProfile", "-NonInteractive", "-Command", script].map(str::to_string);
        let output = run("powershell.exe", &args, Duration::from_secs(20))
            .await
            .expect("hidden helper should complete and return its output");
        assert_eq!(output.trim(), "0", "helper must not own a console window");
    }
}
