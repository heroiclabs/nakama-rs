use std::convert::Infallible;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};

async fn hello(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    let response = Response::new(Body::from(r#"{"hello":"world"}"#));
    let (mut parts, body) = response.into_parts();
    parts.status = StatusCode::INTERNAL_SERVER_ERROR;
    let response = Response::from_parts(parts, body);
    Ok(response)
}

#[tokio::test]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    simple_logger::SimpleLogger::new().init();

    // For every connection, we must make a `Service` to handle all
    // incoming HTTP requests on said connection.
    let make_svc = make_service_fn(|_conn| {
        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        async { Ok::<_, Infallible>(service_fn(hello)) }
    });

    let addr = ([127, 0, 0, 1], 7350).into();

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}