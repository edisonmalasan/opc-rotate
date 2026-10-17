use crate::models::VpnServer;
use crate::process;
use async_trait::async_trait;
use std::path::Path;
use std::time::{Duration, Instant};

#[async_trait]
pub trait VpnRunner: Send + Sync {
    async fn execute(&self, args: &[String]) -> Result<String, String>;
}

#[derive(Clone)]
pub struct RealVpnRunner {
    program: String,
}

impl RealVpnRunner {
    pub fn discover() -> Result<Self, String> {
        let candidates = [
            r"C:\Program Files\SoftEther VPN Client\vpncmd_x64.exe",
            r"C:\Program Files\SoftEther VPN Client\vpncmd.exe",
            r"C:\Program Files (x86)\SoftEther VPN Client\vpncmd.exe",
        ];
        let program = candidates
            .iter()
            .find(|candidate| Path::new(candidate).exists())
            .ok_or("SoftEther VPN Client was not found")?;
        Ok(Self {
            program: (*program).to_string(),
        })
    }
}

#[async_trait]
impl VpnRunner for RealVpnRunner {
    async fn execute(&self, args: &[String]) -> Result<String, String> {
        let mut command = vec!["localhost".into(), "/CLIENT".into(), "/CMD".into()];
        command.extend_from_slice(args);
        let seconds = match args.first().map(String::as_str) {
            Some("AccountDisconnect" | "AccountDelete") => 35,
            Some("AccountConnect") => 20,
            Some("AccountStatusGet") => 4,
            _ => 12,
        };
        process::run(&self.program, &command, Duration::from_secs(seconds))
            .await
            .map_err(|error| {
                let start = error
                    .find("Error occurred.")
                    .or_else(|| error.find("Error (Error Code"));
                match start {
                    Some(index) => error[index..]
                        .lines()
                        .take(3)
                        .map(str::trim)
                        .collect::<Vec<_>>()
                        .join(" "),
                    None => error,
                }
            })
    }
}

pub struct SoftEther<R: VpnRunner> {
    runner: R,
    adapter: String,
}

impl<R: VpnRunner> SoftEther<R> {
    pub fn new(runner: R, adapter: impl Into<String>) -> Self {
        Self {
            runner,
            adapter: adapter.into(),
        }
    }

    async fn command(&self, args: Vec<String>) -> Result<String, String> {
        let output = self.runner.execute(&args).await?;
        if output.contains("The command completed successfully.") {
            Ok(output)
        } else {
            Err(output
                .lines()
                .find(|line| line.contains("Error") || line.contains("error"))
                .unwrap_or("SoftEther command failed")
                .trim()
                .to_string())
        }
    }

    pub async fn adapter_exists(&self) -> Result<bool, String> {
        let output = self.command(vec!["NicList".into()]).await?;
        Ok(output.lines().any(|line| {
            line.split_once('|').is_some_and(|(label, value)| {
                label.trim() == "Virtual Network Adapter Name"
                    && value.trim().eq_ignore_ascii_case(&self.adapter)
            })
        }))
    }

    pub async fn connected_profiles(&self) -> Result<Vec<String>, String> {
        let output = self.command(vec!["AccountList".into()]).await?;
        Ok(parse_connected_profiles(&output))
    }

    pub async fn list_profiles(&self) -> Result<Vec<String>, String> {
        let output = self.command(vec!["AccountList".into()]).await?;
        Ok(profile_names(&output))
    }

    pub async fn connect(
        &self,
        server: &VpnServer,
        profile: &str,
        timeout: Duration,
    ) -> Result<(), String> {
        if let Err(error) = self.connect_inner(server, profile, timeout).await {
            if let Err(cleanup_error) = self.cleanup(profile).await {
                return Err(format!("{error}; cleanup failed: {cleanup_error}"));
            }
            return Err(error);
        }
        Ok(())
    }

    async fn connect_inner(
        &self,
        server: &VpnServer,
        profile: &str,
        timeout: Duration,
    ) -> Result<(), String> {
        if !self.adapter_exists().await? {
            return Err(format!(
                "SoftEther adapter '{}' was not found",
                self.adapter
            ));
        }
        self.command(vec![
            "AccountCreate".into(),
            profile.into(),
            format!("/SERVER:{}:{}", server.ip, server.port),
            "/HUB:VPNGATE".into(),
            "/USERNAME:vpn".into(),
            format!("/NICNAME:{}", self.adapter),
        ])
        .await?;
        self.command(vec![
            "AccountPasswordSet".into(),
            profile.into(),
            "/PASSWORD:vpn".into(),
            "/TYPE:standard".into(),
        ])
        .await?;
        self.command(vec![
            "AccountRetrySet".into(),
            profile.into(),
            "/NUM:0".into(),
            "/INTERVAL:5".into(),
        ])
        .await?;
        self.command(vec!["AccountStatusHide".into(), profile.into()])
            .await?;
        self.command(vec!["AccountConnect".into(), profile.into()])
            .await?;

        let started = Instant::now();
        while started.elapsed() < timeout {
            if self
                .connected_profiles()
                .await
                .is_ok_and(|profiles| profiles.iter().any(|name| name == profile))
            {
                return Ok(());
            }
            match self
                .command(vec!["AccountStatusGet".into(), profile.into()])
                .await
            {
                Ok(output) if status_connected(&output) => return Ok(()),
                Ok(output) if has_protocol_error(&output) => {
                    return Err(format!(
                        "Connection to {} failed with SoftEther error code 2",
                        server.host
                    ))
                }
                Err(error) if has_protocol_error(&error) => return Err(error),
                _ => {}
            }
            tokio::time::sleep(Duration::from_millis(1200)).await;
        }
        Err(format!("Connection to {} timed out", server.host))
    }

    pub async fn cleanup(&self, profile: &str) -> Result<(), String> {
        let _ = self
            .command(vec!["AccountDisconnect".into(), profile.into()])
            .await;
        let deletion = self
            .command(vec!["AccountDelete".into(), profile.into()])
            .await;
        if self.profile_exists(profile).await? {
            return Err(format!(
                "SoftEther profile {profile} still exists after cancellation: {}",
                deletion
                    .err()
                    .unwrap_or_else(|| "delete did not take effect".into())
            ));
        }
        Ok(())
    }

    pub async fn profile_exists(&self, profile: &str) -> Result<bool, String> {
        let output = self.command(vec!["AccountList".into()]).await?;
        Ok(profile_names(&output).iter().any(|name| name == profile))
    }

    pub async fn status(&self, profile: &str) -> bool {
        if self
            .connected_profiles()
            .await
            .is_ok_and(|profiles| profiles.iter().any(|name| name == profile))
        {
            return true;
        }
        self.command(vec!["AccountStatusGet".into(), profile.into()])
            .await
            .is_ok_and(|output| status_connected(&output))
    }
}

fn has_protocol_error(output: &str) -> bool {
    let text = output.to_ascii_lowercase();
    text.contains("error code 2") || text.contains("error code: 2")
}

fn status_connected(output: &str) -> bool {
    output.lines().any(|line| {
        line.split_once('|').is_some_and(|(label, value)| {
            label.trim().eq_ignore_ascii_case("Session Status")
                && value.contains("Connection Completed")
                && value.contains("Session Established")
        })
    })
}

fn parse_connected_profiles(output: &str) -> Vec<String> {
    let mut current = None;
    let mut connected = Vec::new();
    for line in output.lines() {
        if let Some((label, value)) = line.split_once('|') {
            match label.trim() {
                "VPN Connection Setting Name" => current = Some(value.trim().to_string()),
                "Status" if value.trim().eq_ignore_ascii_case("Connected") => {
                    if let Some(name) = &current {
                        connected.push(name.clone());
                    }
                }
                _ => {}
            }
        }
    }
    connected
}

fn profile_names(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| line.split_once('|'))
        .filter_map(|(label, value)| {
            (label.trim() == "VPN Connection Setting Name").then(|| value.trim().to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct MockRunner {
        responses: Arc<Mutex<VecDeque<Result<String, String>>>>,
        calls: Arc<Mutex<Vec<Vec<String>>>>,
    }

    #[async_trait]
    impl VpnRunner for MockRunner {
        async fn execute(&self, args: &[String]) -> Result<String, String> {
            self.calls.lock().unwrap().push(args.to_vec());
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok("The command completed successfully.".into()))
        }
    }

    fn server() -> VpnServer {
        VpnServer {
            id: "203.0.113.1:443".into(),
            host: "example".into(),
            ip: "203.0.113.1".into(),
            port: 443,
            country: "Japan".into(),
            ping_ms: 10,
            speed_mbps: 100.0,
            sessions: 10,
            score: 0.8,
        }
    }

    #[tokio::test]
    async fn connection_error_cancels_and_deletes_only_own_profile() {
        let success = Ok("The command completed successfully.".into());
        let runner = MockRunner {
            responses: Arc::new(Mutex::new(VecDeque::from(vec![
                Ok("Virtual Network Adapter Name|VPN\nThe command completed successfully.".into()),
                success.clone(),
                success.clone(),
                success.clone(),
                success.clone(),
                Ok("Error (Error Code 2): Protocol error occurred".into()),
                success.clone(),
                success,
            ]))),
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let calls = runner.calls.clone();
        let vpn = SoftEther::new(runner, "VPN");
        assert!(vpn
            .connect(&server(), "OPCRotate_test", Duration::from_millis(1))
            .await
            .is_err());
        let names = calls
            .lock()
            .unwrap()
            .iter()
            .map(|args| args[0].clone())
            .collect::<Vec<_>>();
        assert_eq!(names.last().map(String::as_str), Some("AccountList"));
        assert!(names.contains(&"AccountDelete".into()));
        assert!(names.contains(&"AccountDisconnect".into()));
        assert!(calls
            .lock()
            .unwrap()
            .iter()
            .filter(|args| args.len() > 1)
            .all(|args| !args[1].starts_with("Other")));
    }

    #[tokio::test]
    async fn failed_relay_is_cleaned_before_next_relay_connects() {
        let success = Ok("The command completed successfully.".into());
        let nic =
            Ok("Virtual Network Adapter Name|VPN\nThe command completed successfully.".into());
        let runner = MockRunner {
            responses: Arc::new(Mutex::new(VecDeque::from(vec![
                nic.clone(), success.clone(), success.clone(), success.clone(), success.clone(),
                Ok("Error (Error Code 2): Protocol error occurred".into()),
                success.clone(), success.clone(),
                Ok("The command completed successfully.".into()),
                nic, success.clone(), success.clone(), success.clone(), success.clone(), success.clone(),
                Ok("VPN Connection Setting Name|OPCRotate_good\nStatus|Connected\nThe command completed successfully.".into()),
            ]))),
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let calls = runner.calls.clone();
        let vpn = SoftEther::new(runner, "VPN");
        assert!(vpn
            .connect(&server(), "OPCRotate_bad", Duration::from_millis(1))
            .await
            .is_err());
        assert!(vpn
            .connect(&server(), "OPCRotate_good", Duration::from_secs(2))
            .await
            .is_ok());
        let calls = calls.lock().unwrap();
        let cleanup = calls
            .iter()
            .position(|args| args == &["AccountDelete", "OPCRotate_bad"])
            .unwrap();
        let next = calls
            .iter()
            .position(|args| {
                args == &[
                    "AccountCreate",
                    "OPCRotate_good",
                    "/SERVER:203.0.113.1:443",
                    "/HUB:VPNGATE",
                    "/USERNAME:vpn",
                    "/NICNAME:VPN",
                ]
            })
            .unwrap();
        assert!(cleanup < next);
    }

    #[tokio::test]
    async fn stalled_connection_times_out_and_is_deleted() {
        let success = Ok("The command completed successfully.".into());
        let runner = MockRunner {
            responses: Arc::new(Mutex::new(VecDeque::from(vec![
                Ok("Virtual Network Adapter Name|VPN\nThe command completed successfully.".into()),
                success.clone(),
                success.clone(),
                success.clone(),
                success.clone(),
                success.clone(),
                Ok("Session Status|Connecting\nThe command completed successfully.".into()),
                success.clone(),
                success,
            ]))),
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let calls = runner.calls.clone();
        let vpn = SoftEther::new(runner, "VPN");
        assert!(vpn
            .connect(&server(), "OPCRotate_timeout", Duration::from_millis(1))
            .await
            .unwrap_err()
            .contains("timed out"));
        let calls = calls.lock().unwrap();
        assert_eq!(calls.last().unwrap()[0], "AccountList");
        assert!(calls.iter().any(|args| args[0] == "AccountDelete"));
    }

    #[tokio::test]
    async fn refuses_to_continue_when_cancelled_profile_remains() {
        let success = Ok("The command completed successfully.".into());
        let runner = MockRunner {
            responses: Arc::new(Mutex::new(VecDeque::from(vec![
                Ok("Virtual Network Adapter Name|VPN\nThe command completed successfully.".into()),
                success.clone(), success.clone(), success.clone(), success.clone(),
                Ok("Error (Error Code 2): Protocol error occurred".into()),
                success.clone(), success,
                Ok("VPN Connection Setting Name|OPCRotate_bad\nStatus|Connected\nThe command completed successfully.".into()),
            ]))),
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let vpn = SoftEther::new(runner, "VPN");
        let error = vpn
            .connect(&server(), "OPCRotate_bad", Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(error.contains("cleanup failed"));
        assert!(error.contains("still exists"));
    }

    #[test]
    fn accepts_only_established_session_status() {
        assert!(status_connected(
            "Session Status|Connection Completed (Session Established)"
        ));
        assert!(!status_connected("Session Status|Connecting"));
        assert_eq!(parse_connected_profiles("VPN Connection Setting Name |Other\nStatus |Connected\nVPN Connection Setting Name |Mine\nStatus |Offline"), vec!["Other"]);
    }
}
