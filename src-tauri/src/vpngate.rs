use crate::models::VpnServer;
use base64::Engine;
use csv::StringRecord;
use std::cmp::Ordering;

pub const VPN_GATE_URL: &str = "https://www.vpngate.net/api/iphone/";

pub async fn fetch_servers(client: &reqwest::Client) -> Result<Vec<VpnServer>, String> {
    let content = client
        .get(VPN_GATE_URL)
        .send()
        .await
        .map_err(|e| format!("VPN Gate request failed: {e:?}"))?
        .error_for_status()
        .map_err(|e| format!("VPN Gate returned an error: {e}"))?
        .text()
        .await
        .map_err(|e| format!("VPN Gate response was unreadable: {e}"))?;
    let servers = parse_servers(&content);
    if servers.is_empty() {
        Err("VPN Gate returned no eligible TCP relays".into())
    } else {
        Ok(servers)
    }
}

pub fn parse_servers(content: &str) -> Vec<VpnServer> {
    let rows = content
        .lines()
        .filter(|line| !line.starts_with('*') && !line.starts_with('#') && !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(rows.as_bytes());
    let mut servers = Vec::new();
    for row in reader.records().flatten() {
        if let Some(server) = parse_row(&row) {
            servers.push(server);
        }
    }
    servers.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    servers
}

fn parse_row(row: &StringRecord) -> Option<VpnServer> {
    if row.len() < 15 {
        return None;
    }
    let ping_ms: u32 = row.get(3)?.parse().ok()?;
    let speed_bps: f64 = row.get(4)?.parse().ok()?;
    let speed_mbps = speed_bps / 1_000_000.0;
    if ping_ms == 0 || ping_ms > 250 || speed_mbps < 20.0 {
        return None;
    }
    let ip = row.get(1)?.trim();
    let host = row.get(0)?.trim();
    let country = row.get(5)?.trim();
    let sessions = row.get(7)?.parse().unwrap_or(0);
    let port = tcp_port(row.get(14)?)?;
    let latency_score = 1.0 / (1.0 + ping_ms as f64 / 50.0);
    let speed_score = (1.0 + speed_mbps).ln() / 1001.0_f64.ln();
    let score = 0.6 * latency_score + 0.4 * speed_score;
    Some(VpnServer {
        id: format!("{ip}:{port}"),
        host: host.to_string(),
        ip: ip.to_string(),
        port,
        country: country.to_string(),
        ping_ms,
        speed_mbps,
        sessions,
        score,
    })
}

fn tcp_port(encoded_config: &str) -> Option<u16> {
    let data = base64::engine::general_purpose::STANDARD
        .decode(encoded_config.trim())
        .ok()?;
    let config = String::from_utf8(data).ok()?;
    let is_tcp = config.lines().any(|line| {
        let mut words = line.split_whitespace();
        words.next() == Some("proto") && words.next().is_some_and(|p| p.starts_with("tcp"))
    });
    if !is_tcp {
        return None;
    }
    config.lines().find_map(|line| {
        let words = line.split_whitespace().collect::<Vec<_>>();
        (words.len() >= 3 && words[0] == "remote")
            .then(|| words[2].parse::<u16>().ok())
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(host: &str, ping: u32, speed: u64, proto: &str) -> String {
        let config = format!("client\nproto {proto}\nremote 203.0.113.4 1464\n");
        let encoded = base64::engine::general_purpose::STANDARD.encode(config);
        format!(
            "{host},203.0.113.4,100,{ping},{speed},Japan,JP,20,1,2,3,2weeks,test,message,{encoded}"
        )
    }

    #[test]
    fn parses_tcp_port_and_ranks_servers() {
        let input = format!(
            "*vpn_servers\n#HostName,IP,...\n{}\n{}\n*\n",
            row("slow", 90, 25_000_000, "tcp"),
            row("fast", 10, 250_000_000, "tcp")
        );
        let servers = parse_servers(&input);
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].host, "fast");
        assert_eq!(servers[0].port, 1464);
    }

    #[test]
    fn rejects_udp_only_and_missing_quality() {
        let input = format!(
            "{}\n{}\n{}",
            row("udp", 10, 200_000_000, "udp"),
            row("no-ping", 0, 200_000_000, "tcp"),
            row("slow", 10, 1_000_000, "tcp")
        );
        assert!(parse_servers(&input).is_empty());
    }
}
