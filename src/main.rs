use std::time::Duration;

use axum::{
    body::{Body, Bytes},
    http::Response,
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use tokio::{
    sync::{broadcast, mpsc},
    time::sleep,
};
use tokio_stream::wrappers::ReceiverStream;

#[derive(Clone)]
enum Msg {
    Tick,
    Tock,
}

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel::<Msg>(10);

    tokio::spawn({
        let tx = tx.clone();
        async move {
            loop {
                tx.send(Msg::Tick).ok();
                sleep(Duration::from_millis(150)).await;
                tx.send(Msg::Tock).ok();
                sleep(Duration::from_millis(850)).await;
            }
        }
    });

    let app = Router::new()
        .route("/", get(serve_stream))
        .layer(Extension(tx));
    axum::Server::bind(&"[::]:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn serve_stream(Extension(tx): Extension<broadcast::Sender<Msg>>) -> impl IntoResponse {
    let mut rx = tx.subscribe();

    let (s_tx, s_rx) = mpsc::channel(1);

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => match msg {
                    Msg::Tick => s_tx.send(Ok(Bytes::from_static(b"Tick\n"))).await.unwrap(),
                    Msg::Tock => s_tx.send(Ok(Bytes::from_static(b"Tock\n"))).await.unwrap(),
                },
                Err(err) => {
                    println!("body error: {err}");
                    return;
                }
            }
        }
    });

    let body = Body::wrap_stream::<_, _, std::io::Error>(ReceiverStream::new(s_rx));
    let mut res = Response::new(body);
    res.headers_mut()
        .insert("server", "super-sample-code".try_into().unwrap());
    res
}
