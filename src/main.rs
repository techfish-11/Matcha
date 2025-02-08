use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use tokio::runtime::Runtime;
use std::net::SocketAddr;
use std::fs;
use std::path::Path;
use toml;
use hyper::header::HeaderValue;
use serde::Deserialize;
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;

#[derive(Debug, Deserialize, Clone)]
struct Config {
    server: ServerConfig,
    log: LogConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct ServerConfig {
    port: u16,
    host: String,
    root_dir: String,
    index_file: String,
}

#[derive(Debug, Deserialize, Clone)]
struct LogConfig {
    log_path: String,
    log_format: String,
}

fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_contents = fs::read_to_string("matcha.conf")?;
    let config: Config = toml::de::from_str(&config_contents)?;
    Ok(config)
}

fn log_request(log_config: &LogConfig, req: &Request<Body>, status: u16) {
    let log_path = &log_config.log_path;
    let log_format = &log_config.log_format;
    let remote_addr = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    let method = req.method().as_str();
    let path = req.uri().path();
    let log_entry = log_format
        .replace("{remote_addr}", remote_addr)
        .replace("{method}", method)
        .replace("{path}", path)
        .replace("{status}", &status.to_string());

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap();
    writeln!(file, "{} - {}", Local::now().format("%Y-%m-%d %H:%M:%S"), log_entry).unwrap();
}

async fn handle_request(req: Request<Body>, config: ServerConfig, log_config: LogConfig) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path();
    let file_path = if path == "/" {
        Path::new(&config.root_dir).join(&config.index_file)
    } else {
        Path::new(&config.root_dir).join(path.strip_prefix("/").unwrap_or(path))
    };

    let mut resp = if file_path.exists() && file_path.is_file() {
        let content = fs::read(file_path).unwrap_or_else(|_| vec![]);
        Response::new(Body::from(content))
    } else {
        let not_found_path = Path::new("./alerts-public/404.html");
        if not_found_path.exists() && not_found_path.is_file() {
            let content = fs::read(not_found_path).unwrap_or_else(|_| vec![]);
            Response::new(Body::from(content))
        } else {
            Response::new(Body::from("404 Not Found"))
        }
    };
    resp.headers_mut().insert("X-Powered-by", HeaderValue::from_static("Matcha/0.1"));
    log_request(&log_config, &req, resp.status().as_u16());
    Ok(resp)
}

fn main() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let config = load_config().expect("Failed to load config");

        let make_svc = make_service_fn(|_conn| {
            let server_config = config.server.clone();
            let log_config = config.log.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let server_config = server_config.clone();
                    let log_config = log_config.clone();
                    handle_request(req, server_config, log_config)
                }))
            }
        });

        let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
            .parse()
            .unwrap();
        println!("Listening on http://{}", addr);

        let server = Server::bind(&addr).serve(make_svc);

        if let Err(e) = server.await {
            eprintln!("Server error: {}", e);
        }
    });
}
