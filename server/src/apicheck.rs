use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const WARMUP_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_BODY: usize = 1 << 20;

#[derive(Debug)]
pub struct Target {
    host: String,
    port: u16,
    prefix: String,
    tls: bool,
}

pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: Option<String>,
}

struct Reply {
    status: u16,
    body: Option<Value>,
    location: Option<String>,
}

impl Reply {
    fn object(&self) -> Option<&serde_json::Map<String, Value>> {
        self.body.as_ref().and_then(|value| value.as_object())
    }

    fn field(&self, key: &str) -> Option<&Value> {
        self.object().and_then(|map| map.get(key))
    }

    fn number(&self, key: &str) -> Option<i64> {
        let value = self.field(key)?;
        value.as_i64().or_else(|| {
            value
                .as_f64()
                .filter(|number| number.fract() == 0.0)
                .map(|number| number as i64)
        })
    }

    fn id(&self, key: &str) -> Option<&Value> {
        self.field(key)
    }

    fn text(&self, key: &str) -> Option<&str> {
        self.field(key).and_then(|value| value.as_str())
    }

    fn summary(&self) -> String {
        match &self.body {
            Some(body) => format!("{} {}", self.status, truncate(&body.to_string(), 160)),
            None => format!("{} (sem corpo)", self.status),
        }
    }
}

fn render_id(id: Option<&Value>) -> String {
    match id {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Null) | None => "None".to_string(),
        Some(other) => other.to_string(),
    }
}

fn same_json(expected: &Value, got: &Value) -> bool {
    match (expected, got) {
        (Value::Number(want), Value::Number(have)) => match (want.as_f64(), have.as_f64()) {
            (Some(want), Some(have)) => want == have,
            _ => false,
        },
        (Value::Array(want), Value::Array(have)) => {
            want.len() == have.len()
                && want
                    .iter()
                    .zip(have.iter())
                    .all(|(want, have)| same_json(want, have))
        }
        (Value::Object(want), Value::Object(have)) => {
            want.len() == have.len()
                && want.iter().all(|(key, want)| {
                    have.get(key).map(|have| same_json(want, have)).unwrap_or(false)
                })
        }
        (want, have) => want == have,
    }
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let head: String = text.chars().take(limit).collect();
    format!("{head}…")
}

fn looks_public(authority: &str) -> bool {
    if authority.contains(':') {
        return false;
    }
    if !authority.contains('.') {
        return false;
    }
    authority
        .split('.')
        .last()
        .map(|suffix| suffix.chars().any(|letter| letter.is_ascii_alphabetic()))
        .unwrap_or(false)
}

pub fn parse_target(raw: &str) -> Result<Target, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let lowered = trimmed.to_lowercase();
    let (rest, scheme) = if lowered.starts_with("https://") {
        (&trimmed[8..], Some(true))
    } else if lowered.starts_with("http://") {
        (&trimmed[7..], Some(false))
    } else {
        (trimmed, None)
    };
    if rest.contains('@') {
        return Err("a URL não pode ter usuário e senha".to_string());
    }

    let split = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..split];
    let prefix = rest[split..].trim_end_matches('/').to_string();
    if authority.is_empty() {
        return Err("informe o endereço da sua API, por exemplo http://127.0.0.1:8000".to_string());
    }

    let tls = scheme.unwrap_or_else(|| looks_public(authority));
    let fallback = if tls { 443 } else { 80 };
    let (host, port) = if let Some(end) = authority.strip_prefix('[').and_then(|_| authority.find(']')) {
        let host = authority[1..end].to_string();
        let port = match authority[end + 1..].strip_prefix(':') {
            Some(port) => port
                .parse::<u16>()
                .map_err(|_| format!("porta inválida em `{authority}`"))?,
            None => fallback,
        };
        (host, port)
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) => (
                host.to_string(),
                port.parse::<u16>()
                    .map_err(|_| format!("porta inválida em `{authority}`"))?,
            ),
            None => (authority.to_string(), fallback),
        }
    };

    if host.is_empty() {
        return Err("informe o endereço da sua API, por exemplo http://127.0.0.1:8000".to_string());
    }
    Ok(Target { host, port, prefix, tls })
}

const MAX_REDIRECTS: usize = 3;

async fn call(target: &Target, method: &str, path: &str, body: Option<Value>) -> Result<Reply, String> {
    let mut method = method.to_string();
    let mut path = format!("{}{}", target.prefix, path);
    let mut body = body;

    for _ in 0..=MAX_REDIRECTS {
        let reply = send(target, &method, &path, body.clone()).await?;
        let Some(location) = reply.location.clone() else {
            return Ok(reply);
        };
        let follows = match (reply.status, method.as_str()) {
            (301 | 302 | 303, _) => true,
            (307 | 308, "GET") => true,
            _ => false,
        };
        if !follows {
            return Ok(reply);
        }
        if matches!(reply.status, 301 | 302 | 303) {
            method = "GET".to_string();
            body = None;
        }
        path = redirect_path(target, &location);
    }
    Err("sua API entrou em um laço de redirecionamentos".to_string())
}

fn redirect_path(target: &Target, location: &str) -> String {
    let lowered = location.to_lowercase();
    if !lowered.starts_with("http://") && !lowered.starts_with("https://") {
        return location.to_string();
    }
    let rest = &location[location.find("//").map(|at| at + 2).unwrap_or(0)..];
    match rest.find('/') {
        Some(at) => rest[at..].to_string(),
        None => target.prefix.clone(),
    }
}

fn tls_config() -> Arc<ClientConfig> {
    let mut roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    for certificate in rustls_native_certs::load_native_certs().certs {
        let _ = roots.add(certificate);
    }
    Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

async fn exchange<S>(mut stream: S, request: String) -> Result<Vec<u8>, String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|problem| format!("falha ao enviar: {problem}"))?;

    let mut raw = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|problem| format!("falha ao ler: {problem}"))?;
        if read == 0 {
            break;
        }
        raw.extend_from_slice(&chunk[..read]);
        if raw.len() > MAX_BODY {
            return Err("a resposta da sua API é grande demais".to_string());
        }
        if complete(&raw) {
            break;
        }
    }
    Ok(raw)
}

async fn send(target: &Target, method: &str, path: &str, body: Option<Value>) -> Result<Reply, String> {
    send_with(target, method, path, body, REQUEST_TIMEOUT).await
}

async fn send_with(
    target: &Target,
    method: &str,
    path: &str,
    body: Option<Value>,
    budget: Duration,
) -> Result<Reply, String> {
    let payload = body.map(|value| value.to_string());
    let default_port = if target.tls { 443 } else { 80 };
    let host_header = if target.port == default_port {
        target.host.clone()
    } else {
        format!("{}:{}", target.host, target.port)
    };
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\n\
         Host: {host_header}\r\n\
         Accept: application/json\r\n\
         User-Agent: code-fighter\r\n\
         ngrok-skip-browser-warning: true\r\n\
         Connection: close\r\n",
    );
    match &payload {
        Some(text) => {
            request.push_str("Content-Type: application/json\r\n");
            request.push_str(&format!("Content-Length: {}\r\n\r\n", text.len()));
            request.push_str(text);
        }
        None => request.push_str("Content-Length: 0\r\n\r\n"),
    }

    let attempt = async {
        let stream = TcpStream::connect((target.host.as_str(), target.port))
            .await
            .map_err(|problem| format!("não consegui conectar: {problem}"))?;
        if !target.tls {
            return exchange(stream, request).await;
        }
        let name = ServerName::try_from(target.host.clone())
            .map_err(|_| format!("`{}` não é um host válido para https", target.host))?;
        let secured = TlsConnector::from(tls_config())
            .connect(name, stream)
            .await
            .map_err(|problem| format!("falha no https: {problem}"))?;
        exchange(secured, request).await
    };

    let raw = tokio::time::timeout(budget, attempt)
        .await
        .map_err(|_| format!("sua API não respondeu em {} segundos", budget.as_secs()))??;

    parse_reply(&raw)
}

fn header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|window| window == b"\r\n\r\n")
}

fn header_value(head: &str, name: &str) -> Option<String> {
    head.lines()
        .skip(1)
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
}

fn complete(raw: &[u8]) -> bool {
    let Some(split) = header_end(raw) else {
        return false;
    };
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let body = &raw[split + 4..];

    if let Some(length) = header_value(&head, "content-length").and_then(|v| v.parse::<usize>().ok())
    {
        return body.len() >= length;
    }
    if header_value(&head, "transfer-encoding")
        .map(|value| value.to_lowercase().contains("chunked"))
        .unwrap_or(false)
    {
        return dechunk(body).is_ok();
    }
    false
}

fn parse_reply(raw: &[u8]) -> Result<Reply, String> {
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "resposta HTTP incompleta".to_string())?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let body = &raw[split + 4..];

    let status: u16 = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| "resposta HTTP sem status".to_string())?;

    let chunked = header_value(&head, "transfer-encoding")
        .map(|value| value.to_lowercase().contains("chunked"))
        .unwrap_or(false);
    let body = if chunked {
        dechunk(body)?
    } else if let Some(length) =
        header_value(&head, "content-length").and_then(|value| value.parse::<usize>().ok())
    {
        body[..length.min(body.len())].to_vec()
    } else {
        body.to_vec()
    };
    let location = header_value(&head, "location");

    let text = String::from_utf8_lossy(&body).to_string();
    let parsed = if text.trim().is_empty() {
        None
    } else {
        serde_json::from_str::<Value>(text.trim()).ok()
    };
    Ok(Reply {
        status,
        body: parsed,
        location,
    })
}

fn dechunk(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while cursor < body.len() {
        let line_end = body[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| "corpo chunked malformado".to_string())?;
        let size = usize::from_str_radix(
            String::from_utf8_lossy(&body[cursor..cursor + line_end])
                .split(';')
                .next()
                .unwrap_or("")
                .trim(),
            16,
        )
        .map_err(|_| "corpo chunked malformado".to_string())?;
        cursor += line_end + 2;
        if size == 0 {
            break;
        }
        if cursor + size > body.len() {
            return Err("corpo chunked truncado".to_string());
        }
        out.extend_from_slice(&body[cursor..cursor + size]);
        cursor += size + 2;
    }
    Ok(out)
}

struct Sheet {
    checks: Vec<Check>,
}

impl Sheet {
    fn new() -> Self {
        Self { checks: Vec::new() }
    }

    fn record(&mut self, name: &str, ok: bool, got: Option<String>) {
        self.checks.push(Check {
            name: name.to_string(),
            ok,
            detail: if ok { None } else { got },
        });
    }

    fn passed(&self) -> usize {
        self.checks.iter().filter(|check| check.ok).count()
    }
}

pub async fn run(raw_url: &str) -> Result<(Vec<Check>, usize), String> {
    let target = parse_target(raw_url)?;
    let _ = send_with(&target, "GET", "/", None, WARMUP_TIMEOUT).await;
    let mut sheet = Sheet::new();

    let maria = call(
        &target,
        "POST",
        "/contas",
        Some(json!({ "titular": "Maria Silva", "saldo_inicial": 50000 })),
    )
    .await?;
    sheet.record(
        "cria conta e devolve 201",
        maria.status == 201,
        Some(maria.summary()),
    );
    sheet.record(
        "resposta traz id, titular e saldo",
        ["id", "titular", "saldo"]
            .iter()
            .all(|key| maria.field(key).is_some()),
        Some(maria.summary()),
    );
    sheet.record(
        "saldo inicial é respeitado",
        maria.number("saldo") == Some(50000),
        Some(maria.summary()),
    );
    let id_maria = maria.id("id").cloned();
    let path_maria = render_id(id_maria.as_ref());

    let reply = call(
        &target,
        "POST",
        "/contas",
        Some(json!({ "titular": "Joao Souza", "saldo_inicial": -1 })),
    )
    .await?;
    sheet.record(
        "saldo_inicial negativo devolve 422",
        reply.status == 422,
        Some(reply.summary()),
    );

    let reply = call(
        &target,
        "POST",
        "/contas",
        Some(json!({ "titular": "", "saldo_inicial": 100 })),
    )
    .await?;
    sheet.record(
        "titular vazio devolve 422",
        reply.status == 422,
        Some(reply.summary()),
    );

    let conta = call(&target, "GET", &format!("/contas/{path_maria}"), None).await?;
    sheet.record(
        "consulta conta existente devolve 200",
        conta.status == 200,
        Some(conta.summary()),
    );
    sheet.record(
        "dados batem com o que foi criado",
        conta.text("titular") == Some("Maria Silva") && conta.number("saldo") == Some(50000),
        Some(conta.summary()),
    );

    let reply = call(&target, "GET", "/contas/99999", None).await?;
    sheet.record(
        "conta inexistente devolve 404",
        reply.status == 404,
        Some(reply.summary()),
    );
    sheet.record(
        "404 traz detail 'conta nao encontrada'",
        reply.text("detail") == Some("conta nao encontrada"),
        Some(reply.summary()),
    );

    let path = format!("/contas/{path_maria}/transacoes");
    let reply = call(
        &target,
        "POST",
        &path,
        Some(json!({ "tipo": "CREDITO", "valor": 15000 })),
    )
    .await?;
    sheet.record("crédito devolve 200", reply.status == 200, Some(reply.summary()));
    sheet.record(
        "crédito soma ao saldo (50000 + 15000 = 65000)",
        reply.number("saldo") == Some(65000),
        Some(reply.summary()),
    );

    let reply = call(
        &target,
        "POST",
        &path,
        Some(json!({ "tipo": "DEBITO", "valor": 5000 })),
    )
    .await?;
    sheet.record("débito devolve 200", reply.status == 200, Some(reply.summary()));
    sheet.record(
        "débito subtrai do saldo (65000 - 5000 = 60000)",
        reply.number("saldo") == Some(60000),
        Some(reply.summary()),
    );

    let reply = call(
        &target,
        "POST",
        &path,
        Some(json!({ "tipo": "DEBITO", "valor": 999999 })),
    )
    .await?;
    sheet.record(
        "débito maior que o saldo devolve 400",
        reply.status == 400,
        Some(reply.summary()),
    );
    sheet.record(
        "400 traz detail 'saldo insuficiente'",
        reply.text("detail") == Some("saldo insuficiente"),
        Some(reply.summary()),
    );

    let conta = call(&target, "GET", &format!("/contas/{path_maria}"), None).await?;
    sheet.record(
        "débito recusado não alterou o saldo",
        conta.number("saldo") == Some(60000),
        Some(conta.summary()),
    );

    let reply = call(
        &target,
        "POST",
        &path,
        Some(json!({ "tipo": "DEBITO", "valor": 60000 })),
    )
    .await?;
    sheet.record(
        "débito exatamente igual ao saldo é aceito",
        reply.status == 200 && reply.number("saldo") == Some(0),
        Some(reply.summary()),
    );

    call(
        &target,
        "POST",
        &path,
        Some(json!({ "tipo": "CREDITO", "valor": 60000 })),
    )
    .await?;

    let reply = call(
        &target,
        "POST",
        "/contas/99999/transacoes",
        Some(json!({ "tipo": "CREDITO", "valor": 100 })),
    )
    .await?;
    sheet.record(
        "transação em conta inexistente devolve 404",
        reply.status == 404,
        Some(reply.summary()),
    );

    let reply = call(
        &target,
        "POST",
        &path,
        Some(json!({ "tipo": "CREDITO", "valor": 0 })),
    )
    .await?;
    sheet.record("valor zero devolve 422", reply.status == 422, Some(reply.summary()));

    let reply = call(
        &target,
        "POST",
        &path,
        Some(json!({ "tipo": "CREDITO", "valor": -50 })),
    )
    .await?;
    sheet.record(
        "valor negativo devolve 422",
        reply.status == 422,
        Some(reply.summary()),
    );

    let reply = call(
        &target,
        "POST",
        &path,
        Some(json!({ "tipo": "PIX", "valor": 100 })),
    )
    .await?;
    sheet.record(
        "tipo inválido devolve 422",
        reply.status == 422,
        Some(reply.summary()),
    );

    let extrato = call(&target, "GET", &format!("/contas/{path_maria}/extrato"), None).await?;
    sheet.record("extrato devolve 200", extrato.status == 200, Some(extrato.summary()));
    sheet.record(
        "extrato traz conta_id e transacoes",
        ["conta_id", "transacoes"]
            .iter()
            .all(|key| extrato.field(key).is_some()),
        Some(extrato.summary()),
    );

    let expected = json!([
        { "tipo": "CREDITO", "valor": 15000 },
        { "tipo": "DEBITO", "valor": 5000 },
        { "tipo": "DEBITO", "valor": 60000 },
        { "tipo": "CREDITO", "valor": 60000 },
    ]);
    sheet.record(
        "extrato tem só as 4 transações aceitas, na ordem certa",
        extrato
            .field("transacoes")
            .map(|got| same_json(&expected, got))
            .unwrap_or(false),
        Some(extrato.summary()),
    );

    let reply = call(&target, "GET", "/contas/99999/extrato", None).await?;
    sheet.record(
        "extrato de conta inexistente devolve 404",
        reply.status == 404,
        Some(reply.summary()),
    );

    let outra = call(
        &target,
        "POST",
        "/contas",
        Some(json!({ "titular": "Joao Souza", "saldo_inicial": 700 })),
    )
    .await?;
    let id_joao = outra.id("id").cloned();
    let path_joao = render_id(id_joao.as_ref());
    sheet.record(
        "segunda conta recebe id diferente da primeira",
        id_joao != id_maria,
        Some(outra.summary()),
    );

    let extrato = call(&target, "GET", &format!("/contas/{path_joao}/extrato"), None).await?;
    sheet.record(
        "conta nova nasce com extrato vazio",
        extrato.field("transacoes") == Some(&json!([])),
        Some(extrato.summary()),
    );

    call(
        &target,
        "POST",
        &format!("/contas/{path_joao}/transacoes"),
        Some(json!({ "tipo": "DEBITO", "valor": 700 })),
    )
    .await?;
    let conta = call(&target, "GET", &format!("/contas/{path_maria}"), None).await?;
    sheet.record(
        "mexer numa conta não afeta a outra",
        conta.number("saldo") == Some(60000),
        Some(conta.summary()),
    );

    let passed = sheet.passed();
    Ok((sheet.checks, passed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_plain_host_and_port() {
        let target = parse_target("http://127.0.0.1:8000/").expect("should parse");
        assert_eq!(target.host, "127.0.0.1");
        assert_eq!(target.port, 8000);
        assert_eq!(target.prefix, "");
    }

    #[test]
    fn keeps_a_path_prefix() {
        let target = parse_target("http://127.0.0.1:8000/api/v1/").expect("should parse");
        assert_eq!(target.prefix, "/api/v1");
    }

    #[test]
    fn accepts_ipv6_and_any_scheme_case() {
        let target = parse_target("HTTP://[::1]:8000").expect("should parse");
        assert_eq!(target.host, "::1");
        assert_eq!(target.port, 8000);
        assert!(parse_target("HTTPS://api.local").expect("should parse").tls);
    }

    #[test]
    fn ids_render_like_python() {
        assert_eq!(render_id(Some(&json!(7))), "7");
        assert_eq!(render_id(Some(&json!("abc-1"))), "abc-1");
        assert_eq!(render_id(None), "None");
    }

    #[test]
    fn floats_and_integers_are_the_same_money() {
        assert!(same_json(&json!([{ "valor": 15000 }]), &json!([{ "valor": 15000.0 }])));
        assert!(!same_json(&json!([{ "valor": 15000 }]), &json!([{ "valor": 15001 }])));
        assert!(!same_json(&json!([{ "valor": 1 }]), &json!([])));
    }

    #[test]
    fn stops_reading_when_content_length_is_satisfied() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\n{\"a\":1}";
        assert!(complete(raw));
        assert!(!complete(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\n{\"a\""));
    }

    #[test]
    fn reads_the_redirect_target() {
        let raw = b"HTTP/1.1 307 Temporary Redirect\r\nlocation: /contas/1/\r\nContent-Length: 0\r\n\r\n";
        let reply = parse_reply(raw).expect("should parse");
        assert_eq!(reply.location.as_deref(), Some("/contas/1/"));
    }

    #[test]
    fn a_bare_hostname_without_a_port_is_treated_as_hosted() {
        let target = parse_target("api.local").expect("should parse");
        assert!(target.tls);
        assert_eq!(target.port, 443);
    }

    #[test]
    fn accepts_https_on_port_443() {
        let target = parse_target("https://contas.vercel.app").expect("should parse");
        assert!(target.tls);
        assert_eq!(target.port, 443);
        assert_eq!(target.host, "contas.vercel.app");
    }

    #[test]
    fn https_keeps_an_explicit_port_and_prefix() {
        let target = parse_target("https://api.local:8443/contas/").expect("should parse");
        assert!(target.tls);
        assert_eq!(target.port, 8443);
        assert_eq!(target.prefix, "/contas");
    }

    #[test]
    fn plain_http_still_defaults_to_eighty() {
        let target = parse_target("http://api.local").expect("should parse");
        assert!(!target.tls);
        assert_eq!(target.port, 80);
    }

    #[test]
    fn a_bare_public_hostname_is_assumed_to_be_https() {
        let tunnel = parse_target("meu-tunel.trycloudflare.com").expect("should parse");
        assert!(tunnel.tls);
        assert_eq!(tunnel.port, 443);
    }

    #[test]
    fn a_bare_host_with_a_port_stays_on_http() {
        for raw in ["127.0.0.1:8000", "192.168.0.10:8000", "localhost:8000"] {
            let target = parse_target(raw).expect("should parse");
            assert!(!target.tls, "{raw} should not be https");
            assert_eq!(target.port, 8000);
        }
    }

    #[test]
    fn an_explicit_scheme_always_wins() {
        assert!(!parse_target("http://meu-tunel.trycloudflare.com").expect("parses").tls);
        assert!(parse_target("https://127.0.0.1:8443").expect("parses").tls);
    }

    #[test]
    fn rejects_credentials() {
        assert!(parse_target("http://user:pass@api.local").is_err());
        assert!(parse_target("https://user:pass@api.local").is_err());
    }

    #[test]
    fn reads_a_chunked_reply() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n7\r\n{\"a\":1}\r\n0\r\n\r\n";
        let reply = parse_reply(raw).expect("should parse");
        assert_eq!(reply.status, 200);
        assert_eq!(reply.number("a"), Some(1));
    }
}
