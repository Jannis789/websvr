use rama::rt::Executor;
use rama::http::server::HttpServer;
use rama::http::service::web::WebService;
use rama::http::client::EasyHttpWebClient;
use rama::http::service::client::HttpClientExt;
use rama::http::BodyExtractExt;
use rama::Context;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    println!("🚀 Server läuft auf http://{}", addr);

    let service = WebService::default().get("/", |_: Context<()>| async { "Hello, Rama!" });
    let server = HttpServer::auto(Executor::default());

    // Server im Hintergrund starten
    let server_handle = tokio::spawn(async move {
        server.listen(addr, service).await.expect("Server konnte nicht starten");
    });

    // HTTP-Client-Request an den eigenen Server
    let client = EasyHttpWebClient::default();
    let url = format!("http://{}/", addr);
    let ctx = Context::default();
    match client.get(url).send(ctx).await {
        Ok(resp) => {
            match resp.try_into_string().await {
                Ok(body) => println!("Client-Response: {}", body),
                Err(e) => eprintln!("Fehler beim Lesen des Bodys: {e}"),
            }
        }
        Err(e) => eprintln!("Fehler beim HTTP-Request: {e}"),
    }

    let _ = server_handle.await;
}