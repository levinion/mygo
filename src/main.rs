use std::path::PathBuf;

use axum::{
    handler::HandlerWithoutStateExt,
    http::Uri,
    response::{Html, IntoResponse},
    Router,
};
use clap::Parser;
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::services::ServeDir;

#[derive(Parser)]
struct Opt {
    #[arg(
        short,
        long,
        default_value = "0.0.0.0",
        help = "Specifies the IP address to bind the server to."
    )]
    addr: String,
    #[arg(
        short,
        long,
        default_value = "8080",
        help = "Specifies the port number for the server."
    )]
    port: u16,
    #[arg(
        default_value = ".",
        help = "Specifies the target directory to serve files from."
    )]
    target: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();
    let socket_addr = format!("{}:{}", &opt.addr, &opt.port);
    let listener = TcpListener::bind(&socket_addr).await?;
    let serve_dir = ServeDir::new(opt.target).not_found_service(fallback.into_service());
    let router = Router::new().fallback_service(serve_dir);
    println!("Serving HTTP on http://{}/ ...", &socket_addr);
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn fallback(uri: Uri) -> impl IntoResponse {
    let opt = Opt::parse();
    let url = PathBuf::from(opt.target).join(uri.path().trim_start_matches('/'));
    let entry = url.read_dir();
    let res = match entry {
        Ok(entry) => entry
            .into_iter()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .filter(|path| {
                !path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string()
                    .starts_with('.')
            })
            .map(|path| {
                format!(
                    "<a href='{}'>{}</a>",
                    path.file_name().unwrap().to_str().unwrap(),
                    path.display(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n<br/>\n"),
        Err(_) => "".to_string(),
    };
    let temp = format!(
        r#"
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>{}</title>
        <link
  rel="stylesheet"
  href="https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.min.css"
/>
    </head>
    <body>
    {}
    </body>
    </html>
    "#,
        url.to_str().unwrap(),
        res
    );
    Html(temp)
}

// https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
