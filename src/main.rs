mod routes;

use routes::register_routes;

// TODO (Pen):
// - Logging
// - App state (for example: the birdeye API URL and the secrets)
// - Be careful what you log when it comes to secrets!
#[tokio::main]
async fn main() {
    let app = register_routes();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
