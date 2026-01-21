mod route;
mod pages;
mod util;
mod components;
use route::router::router;
use dotenv::dotenv;
use std::env;
use rama::rt::Executor;
use rama::http::server::HttpServer;
use rama::http::client::EasyHttpWebClient;
use rama::http::service::client::HttpClientExt;
use rama::http::BodyExtractExt;

#[tokio::main]
async fn main() {

    dotenv().ok();
    let host = env::var("SERVER_HOST").expect("SERVER_HOST fehlt in .env");
    let port = env::var("SERVER_PORT").expect("SERVER_PORT fehlt in .env");
    let addr = format!("{}:{}", host, port);
    println!("🚀 Server läuft auf http://{}", addr);

    let service = router();
    let server = HttpServer::auto(Executor::default());

    // Server im Hintergrund starten

    let addr_server = addr.clone();
    let server_handle = tokio::spawn(async move {
        server.listen(addr_server, service).await.expect("Server konnte nicht starten");
    });

    // HTTP-Client-Request an den eigenen Server
    let client = EasyHttpWebClient::default();
    let url = format!("http://{}/", addr);
    match client.get(url).send().await {
        Ok(resp) => {
            match resp.try_into_string().await {
                Ok(_body) => println!("Client responded with body!"),
                Err(e) => eprintln!("Fehler beim Lesen des Bodys: {e}"),
            }
        }
        Err(e) => eprintln!("Fehler beim HTTP-Request: {e}"),
    }

    let _ = server_handle.await;
}