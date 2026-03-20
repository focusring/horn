use axum::Router;
use std::net::SocketAddr;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let static_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static");
    let wasm_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("horn-wasm/pkg");

    let app = Router::new()
        .nest_service("/wasm", ServeDir::new(&wasm_dir))
        .fallback_service(ServeDir::new(&static_dir).append_index_html_on_directories(true));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Horn GUI listening on http://{addr}");
    println!("  Serving static files from: {}", static_dir.display());
    println!("  Serving WASM from: {}", wasm_dir.display());

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
