use hyper::{Body, Request, Response, Server, Method};
use hyper::service::{make_service_fn, service_fn};
use tokio::runtime::Runtime;
use std::net::SocketAddr;
use hyper::header::HeaderValue;

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match req.method() {
        &Method::GET => {
            let mut resp = Response::new(Body::from("Hello, Matcha WebServer!"));
            resp.headers_mut().insert("Server", HeaderValue::from_static("Matcha"));
            Ok(resp)
        }
        _ => {
            let mut resp = Response::new(Body::from("Method Not Allowed"));
            resp.headers_mut().insert("Server", HeaderValue::from_static("Matcha/0.1"));
            Ok(resp)
        }
    }
}

fn main() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let make_svc = make_service_fn(|_conn| async {
            Ok::<_, hyper::Error>(service_fn(handle_request))
        });

        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        println!("Listening on http://{}", addr);

        let server = Server::bind(&addr)
            .serve(make_svc);

        if let Err(e) = server.await {
            eprintln!("Server error: {}", e);
        }
    });
}
