use crate::models::SessionTarget;
use crate::process::now_ms;
use reqwest::Url;
use serde_json::Value;
use std::collections::HashSet;
use std::path::PathBuf;

pub struct OpenCodeClient {
    http: reqwest::Client,
    base: Url,
    password: String,
    pub service_pid: u32,
}

impl OpenCodeClient {
    pub fn discover(http: reqwest::Client) -> Result<Self, String> {
        let home = dirs::home_dir().ok_or("Home directory not found")?;
        let path: PathBuf = home
            .join(".local")
            .join("state")
            .join("opencode")
            .join("service.json");
        let content = std::fs::read_to_string(path)
            .map_err(|_| "OpenCode shared service is not running".to_string())?;
        let json: Value = serde_json::from_str(&content)
            .map_err(|_| "OpenCode service state is invalid".to_string())?;
        let url = json["url"]
            .as_str()
            .ok_or("OpenCode service URL is missing")?;
        let base = Url::parse(url).map_err(|e| format!("Invalid OpenCode service URL: {e}"))?;
        if base.scheme() != "http"
            || !matches!(base.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
        {
            return Err("OpenCode service must be on loopback".into());
        }
        let password = json["password"]
            .as_str()
            .ok_or("OpenCode service password is missing")?
            .to_string();
        let service_pid = json["pid"]
            .as_u64()
            .and_then(|pid| u32::try_from(pid).ok())
            .filter(|pid| *pid > 0)
            .ok_or("OpenCode service PID is missing")?;
        Ok(Self {
            http,
            base,
            password,
            service_pid,
        })
    }

    async fn get(&self, path: &str) -> Result<Value, String> {
        let url = self.base.join(path).map_err(|e| e.to_string())?;
        self.http
            .get(url)
            .basic_auth("opencode", Some(&self.password))
            .send()
            .await
            .map_err(|e| format!("OpenCode service request failed: {e}"))?
            .error_for_status()
            .map_err(|e| format!("OpenCode service returned an error: {e}"))?
            .json()
            .await
            .map_err(|e| format!("OpenCode response was invalid: {e}"))
    }

    pub async fn active_ids(&self) -> Result<HashSet<String>, String> {
        let value = self.get("/api/session/active").await?;
        value["data"]
            .as_object()
            .map(|map| map.keys().cloned().collect())
            .ok_or("OpenCode did not return active sessions".into())
    }

    pub async fn list_targets(
        &self,
        presence: &crate::presence::Presence,
        previously_active: &HashSet<String>,
    ) -> Result<Vec<SessionTarget>, String> {
        let active = self.active_ids().await?;
        let live = active
            .union(&presence.open)
            .cloned()
            .collect::<HashSet<_>>();
        let mut ids = live.iter().cloned().collect::<Vec<_>>();
        ids.sort();
        let mut previously_active_ids = previously_active
            .difference(&live)
            .cloned()
            .collect::<Vec<_>>();
        previously_active_ids.sort();
        ids.extend(previously_active_ids);
        let mut targets = Vec::new();
        for id in ids {
            if presence.orca.contains(&id)
                || presence
                    .targets
                    .iter()
                    .any(|target| target.session_id.as_deref() == Some(id.as_str()))
            {
                continue;
            }
            let session = match self.get(&format!("/api/session/{id}")).await {
                Ok(value) => value["data"].clone(),
                Err(error) if live.contains(&id) => return Err(error),
                Err(_) => continue,
            };
            if session["id"].as_str() != Some(id.as_str()) {
                return Err(format!("OpenCode returned the wrong session for {id}"));
            }
            let project = session["location"]["directory"].as_str().unwrap_or("");
            let updated_at = session["time"]["updated"].as_u64().unwrap_or_else(now_ms);
            let path = format!("/api/session/{id}/message?limit=1&order=desc");
            let latest = self.get(&path).await.ok();
            let message = latest
                .as_ref()
                .and_then(|v| v["data"].as_array())
                .and_then(|a| a.first());
            let error = if active.contains(&id) {
                None
            } else {
                message.and_then(recovery_error)
            };
            if !include_other_session(
                live.contains(&id),
                previously_active.contains(&id),
                error.is_some(),
            ) {
                continue;
            }
            let error_key = error
                .as_ref()
                .and_then(|_| message.and_then(|m| m["id"].as_str()))
                .map(|message_id| format!("api:{id}:{message_id}"));
            targets.push(SessionTarget {
                id: format!("api:{id}"),
                source: "other".into(),
                title: session["title"]
                    .as_str()
                    .unwrap_or("Untitled session")
                    .into(),
                project: project.into(),
                state: if error.is_some() {
                    "rate-limited"
                } else if active.contains(&id) {
                    "busy"
                } else {
                    "idle"
                }
                .into(),
                last_error: error,
                error_key,
                prompt_override: None,
                updated_at,
                session_id: Some(id),
                ..Default::default()
            });
        }
        Ok(targets)
    }

    pub async fn prompt(&self, id: &str, prompt: &str) -> Result<String, String> {
        let url = self
            .base
            .join(&format!("/api/session/{id}/prompt"))
            .map_err(|e| e.to_string())?;
        let response = self
            .http
            .post(url)
            .basic_auth("opencode", Some(&self.password))
            .json(&serde_json::json!({"text":prompt}))
            .send()
            .await
            .map_err(|e| format!("OpenCode prompt delivery is uncertain: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "OpenCode rejected the prompt (HTTP {})",
                response.status()
            ));
        }
        let value: Value = response.json().await.map_err(|e| {
            format!("OpenCode accepted a prompt but its receipt was unreadable: {e}")
        })?;
        Ok(value["data"]["id"].as_str().unwrap_or("accepted").into())
    }
}

fn include_other_session(active: bool, previously_active: bool, rate_limited: bool) -> bool {
    active || (previously_active && rate_limited)
}

fn recovery_error(message: &Value) -> Option<String> {
    if message["type"].as_str() != Some("idle") || message["outcome"].as_str() != Some("failed") {
        return None;
    }
    let error = &message["error"];
    let status = error["status"].as_u64();
    let kind = error["type"].as_str().unwrap_or("");
    let text = error["message"].as_str().unwrap_or("");
    let normalized = text
        .replace('’', "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let free_tier_gate =
        normalized.contains("opencode's free tier can only be used from within opencode.");
    if free_tier_gate
        || (status == Some(429)
            && (kind == "provider.rate-limit"
                || kind == "provider.quota"
                || text.to_ascii_lowercase().contains("rate limit")))
    {
        Some(text.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn mock_service(
        reply: Option<&'static str>,
    ) -> (OpenCodeClient, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let read = socket.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if let Some(header_end) = request.windows(4).position(|chunk| chunk == b"\r\n\r\n")
                {
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= header_end + 4 + length {
                        break;
                    }
                }
            }
            if let Some(body) = reply {
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                socket.write_all(response.as_bytes()).unwrap();
            }
            String::from_utf8_lossy(&request).to_string()
        });
        let client = OpenCodeClient {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap(),
            base: Url::parse(&format!("http://{address}/")).unwrap(),
            password: "test-password".into(),
            service_pid: 10,
        };
        (client, handle)
    }

    #[test]
    fn free_tier_client_gate_triggers_recovery_only_from_failed_provider_events() {
        let text = "OpenCode's free tier can only be used from within OpenCode. Download it at [https://opencode.ai](https://opencode.ai) and keep building.";
        for status in [serde_json::json!(403), serde_json::Value::Null] {
            let message = serde_json::json!({"type":"idle","outcome":"failed","error":{"type":"provider.unknown","status":status,"message":text}});
            assert_eq!(recovery_error(&message).as_deref(), Some(text));
        }
        let wrapped = "OpenCode’s free tier can only be\nused from within OpenCode. Download it at https://opencode.ai";
        assert!(recovery_error(
            &serde_json::json!({"type":"idle","outcome":"failed","error":{"message":wrapped}})
        )
        .is_some());
        assert!(
            recovery_error(&serde_json::json!({"type":"assistant","error":{"message":text}}))
                .is_none()
        );
        assert!(recovery_error(
            &serde_json::json!({"type":"idle","outcome":"completed","error":{"message":text}})
        )
        .is_none());
        assert!(recovery_error(&serde_json::json!({"type":"idle","outcome":"failed","error":{"status":403,"message":"Permission denied"}})).is_none());
    }

    #[test]
    fn detects_actual_failed_429_event() {
        let message = serde_json::json!({"type":"idle","outcome":"failed","error":{"type":"provider.quota","status":429,"message":"Rate limit exceeded"}});
        assert_eq!(
            recovery_error(&message).as_deref(),
            Some("Rate limit exceeded")
        );
        assert!(
            recovery_error(&serde_json::json!({"type":"assistant","error":{"status":429}}))
                .is_none()
        );
        assert!(recovery_error(&serde_json::json!({"type":"idle","outcome":"failed","error":{"status":500,"message":"server error"}})).is_none());
    }

    #[tokio::test]
    async fn excludes_history_but_keeps_an_observed_rate_limit() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for _ in 0..5 {
                let (mut socket, _) = listener.accept().unwrap();
                let mut buffer = [0u8; 4096];
                let count = socket.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]);
                let body = if request.starts_with("GET /api/session/active ") {
                    r#"{"data":{"live":{}}}"#
                } else if request.starts_with("GET /api/session/live ") {
                    r#"{"data":{"id":"live","title":"Live","location":{"directory":"C:/live"},"time":{"updated":100}}}"#
                } else if request.starts_with("GET /api/session/failed ") {
                    r#"{"data":{"id":"failed","title":"Failed","location":{"directory":"C:/failed"},"time":{"updated":101}}}"#
                } else if request.starts_with("GET /api/session/failed/message?") {
                    r#"{"data":[{"id":"failure-1","type":"idle","outcome":"failed","error":{"type":"provider.rate-limit","status":429,"message":"Rate limit exceeded"}}]}"#
                } else {
                    r#"{"data":[]}"#
                };
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                socket.write_all(response.as_bytes()).unwrap();
            }
        });
        let client = OpenCodeClient {
            http: reqwest::Client::new(),
            base: Url::parse(&format!("http://{address}/")).unwrap(),
            password: "test-password".into(),
            service_pid: 10,
        };
        let targets = client
            .list_targets(
                &crate::presence::Presence::default(),
                &HashSet::from(["failed".to_string()]),
            )
            .await
            .unwrap();
        server.join().unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].id, "api:live");
        assert_eq!(targets[0].state, "busy");
        assert_eq!(targets[1].id, "api:failed");
        assert_eq!(targets[1].state, "rate-limited");
        assert_eq!(
            targets[1].error_key.as_deref(),
            Some("api:failed:failure-1")
        );
    }

    #[tokio::test]
    async fn shows_open_idle_outside_session_in_same_project_and_excludes_exact_orca_id() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..3 {
                let (mut socket, _) = listener.accept().unwrap();
                let mut buffer = [0u8; 4096];
                let count = socket.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]).to_string();
                let body = if request.starts_with("GET /api/session/active ") {
                    r#"{"data":{"orca-owned":{"type":"running"}}}"#
                } else if request.starts_with("GET /api/session/outside ") {
                    r#"{"data":{"id":"outside","title":"Open idle work","location":{"directory":"C:/same-project"}}}"#
                } else if request.starts_with("GET /api/session/outside/message?") {
                    r#"{"data":[]}"#
                } else {
                    panic!(
                        "Unexpected API request: {}",
                        request.lines().next().unwrap_or("")
                    );
                };
                requests.push(request);
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).unwrap();
            }
            requests
        });
        let client = OpenCodeClient {
            http: reqwest::Client::new(),
            base: Url::parse(&format!("http://{address}/")).unwrap(),
            password: "test-password".into(),
            service_pid: 10,
        };
        let mut presence = crate::presence::Presence::default();
        presence.open = HashSet::from(["outside".into()]);
        presence.orca = HashSet::from(["orca-owned".into()]);
        presence.clients = 1;
        let targets = client
            .list_targets(&presence, &HashSet::new())
            .await
            .unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "api:outside");
        assert_eq!(targets[0].state, "idle");
        assert_eq!(targets[0].project, "C:/same-project");
        let requests = server.join().unwrap();
        assert!(!requests
            .iter()
            .any(|request| request.starts_with("GET /api/session?")));
    }

    #[tokio::test]
    async fn closed_idle_sessions_are_not_replaced_with_history() {
        let (client, server) = mock_service(Some(r#"{"data":{}}"#));
        let targets = client
            .list_targets(&crate::presence::Presence::default(), &HashSet::new())
            .await
            .unwrap();
        assert!(targets.is_empty());
        assert!(server
            .join()
            .unwrap()
            .starts_with("GET /api/session/active "));
    }

    #[tokio::test]
    async fn prompt_posts_once_and_returns_service_receipt() {
        let (client, server) = mock_service(Some(r#"{"data":{"id":"receipt-123"}}"#));
        assert_eq!(
            client.prompt("session-1", "Continue safely").await.unwrap(),
            "receipt-123"
        );
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /api/session/session-1/prompt HTTP/1.1"));
        assert!(request.contains("\"text\":\"Continue safely\""));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: basic"));
    }

    #[tokio::test]
    async fn dropped_prompt_response_is_reported_as_uncertain() {
        let (client, server) = mock_service(None);
        let error = client
            .prompt("session-2", "Continue safely")
            .await
            .unwrap_err();
        assert!(error.contains("uncertain"));
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /api/session/session-2/prompt HTTP/1.1"));
    }
}
