use std::fs::File;
use std::io::{BufReader, Result as StdIoResult};
use actix_web::{App, HttpResponse, HttpServer, web, error, HttpRequest};
use actix_web::web::{JsonConfig};
use actix_files::NamedFile;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use anyhow::{anyhow, bail, Context, Error, Result as AnyHowResult};
use log::{trace, warn, info};

async fn favicon() -> StdIoResult<NamedFile> {
    NamedFile::open("static/favicon.ico")
}

fn setup_logger() -> Result<(), fern::InitError> {
    use fern::colors::*;
    let mut colors = ColoredLevelConfig::new();

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}

async fn index(req: HttpRequest) -> HttpResponse {
    println!("{:?}", req);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body("<!DOCTYPE html><html><body><p>Welcome!</p></body></html>")
}

fn load_certification_files() -> AnyHowResult<ServerConfig> {
    trace!("loading cert.pem");
    let cert_file = &mut BufReader::new(File::open("cert.pem").context("cert.pem was not found")?);
    let cert_chain = certs(cert_file).unwrap().iter().map(|a| Certificate(a.clone())).collect();
    trace!("loading key.pem");
    let key_file = &mut BufReader::new(File::open("key.pem").context("key.pem was not found")?);
    let mut keys = pkcs8_private_keys(key_file).unwrap().iter().map(|x| PrivateKey(x.clone())).collect::<Vec<_>>();
    if keys.is_empty() {
        bail!("Could not locate PKCS 8 private keys");
    }

    ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, keys.remove(0)).context("failed in with_single_cert ?!?!")
}

const HTTP_PORT: u16 = 11552;
const HTTPS_PORT: u16 = 44300;
static HOST: &'static str = "127.0.0.1";

// TODO: redirect http to https
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    match setup_logger().context("failed to setup logger") {
        Ok(a) => {}
        Err(web) => {
            eprintln!("failed to initialize logger: {:?}", web);
        }
    }

    trace!("starting");
    // load SSL keys
    trace!("Reading config...");
    let mut config = load_certification_files();
    trace!("building HttpServer");
    let mut http_server = HttpServer::new(|| {
        App::new()
            .service(web::resource("/index.html").to(index))
            .service(web::resource("/").route(web::get().to(|| {
                HttpResponse::Found()
                    .header("LOCATION", "/index.html")
                    .finish()
            })))
    });
    trace!("binding https port");
    // it is not required to enable https
    match config {
        Ok(cert_config) => {
            http_server = http_server.bind_rustls(format!("{}:{}", HOST, HTTPS_PORT), cert_config)?;
        }
        Err(error) => {
            warn!("{:?}", error)
        }
    }

    trace!("binding http port");
    let http_server = http_server
        .bind(format!("{}:{}", HOST, HTTP_PORT))?;

    info!("running server...");

    http_server
        .run()
        .await;
    trace!("stopped");
    Ok(())
}

