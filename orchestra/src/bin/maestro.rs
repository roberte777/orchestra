use orchestra::maestro::maestro;

#[tokio::main]
async fn main() {
    let server = maestro();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, server).await.unwrap();
}
