use std::fs::File;
use std::io::{BufReader, Result as StdIoResult};
use actix_web::{App, HttpResponse, HttpServer, web, error, HttpRequest};
use actix_web::web::{JsonConfig};
use actix_files::NamedFile;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use anyhow::{anyhow, bail, Context, Result as AnyHowResult};

async fn favicon() -> StdIoResult<NamedFile> {
    NamedFile::open("static/favicon.ico")
}

async fn index(req: HttpRequest) -> HttpResponse {
    println!("{:?}", req);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body("<!DOCTYPE html><html><body><p>Welcome!</p></body></html>")
}

fn load_certification_files() -> AnyHowResult<ServerConfig> {
    println!("loading cert.pem");
    let cert_file = &mut BufReader::new(File::open("cert.pem").context("cert.pem was not found")?);
    let cert_chain = certs(cert_file).unwrap().iter().map(|a| Certificate(a.clone())).collect();
    println!("loading key.pem");
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("starting");
    // load SSL keys
    let mut config = load_certification_files();

    println!("Reading config...");
    println!("building HttpServer");
    let mut http_server = HttpServer::new(|| {
        App::new()
            .service(web::resource("/index.html").to(index))
            .service(web::resource("/").route(web::get().to(|| {
                HttpResponse::Found()
                    .header("LOCATION", "/index.html")
                    .finish()
            })))
    });
    println!("https");
    // it is not required to enable https
    match config {
        Ok(cert_config) => {
            http_server = http_server.bind_rustls("127.0.0.1:443", cert_config)?;
        }
        Err(error) => {
            eprintln!("{:?}", error)
        }
    }

    println!("http");
    http_server
        .bind("127.0.0.1:80")?
        .run()
        .await;

    println!("stopped");
    Ok(())
}

