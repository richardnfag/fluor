#![deny(warnings)]

pub mod function;
pub mod router;
pub mod trigger;

use function::Function;
use router::Router;
use trigger::Trigger;

use futures::{future, Future, Stream};

use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, Server};

use std::fs::create_dir_all;
use std::path::Path;

fn nanoservices(
    req: Request<Body>,
    router: &Router,
) -> Box<Future<Item = Response<Body>, Error = hyper::Error> + Send> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/function/") => {
            let mut body = String::from("<h1>Functions</h1>");

            router.select().into_iter().for_each(|v| {
                body += format!("<a href=\"{}\">{}</a><br>", v.1.path(), v.1.name()).as_str()
            });

            Box::new(future::ok(
                Response::builder()
                    .status(200)
                    .header("Content-type", "text/html; charset=utf-8")
                    .body(Body::from(body))
                    .unwrap(),
            ))
        }

        (&Method::POST, "/function/") => {
            let router = router.clone();
            Box::new(req.into_body().concat2().map(move |b| {
                let f = match Function::from_json(&b.into_bytes()).map(|f| f.build()) {
                    Some(Some(f)) => f,
                    Some(None) => {
                        return Response::builder()
                            .status(422)
                            .body("Failed build process".into())
                            .unwrap();
                    }
                    None => {
                        return Response::builder()
                            .status(422)
                            .body("JSON error".into())
                            .unwrap();
                    }
                };

                router.insert(f.trigger(), f);

                Response::new("Function Created".into())
            }))
        }

        (&Method::DELETE, "/function/") => {
            let router = router.clone();
            Box::new(req.into_body().concat2().map(move |b| {
                match Function::from_json(&b.into_bytes()) {
                    Some(f) => f.delete(router),
                    None => {
                        return Response::builder()
                            .status(422)
                            .body("JSON error".into())
                            .unwrap();
                    }
                }
            }))
        }

        (_, _) => {
            let (parts, body) = req.into_parts();

            match router.get(&Trigger::new(parts.method.as_str(), parts.uri.path())) {
                Some(f) => Box::new(future::ok(f.run(parts, body))),
                None => Box::new(future::ok(
                    Response::builder().status(404).body(Body::empty()).unwrap(),
                )),
            }
        }
    }
}

fn main() {
    create_dir_all(Path::new("data/")).unwrap();

    let addr = "127.0.0.1:8000".parse().unwrap();

    hyper::rt::run(future::lazy(move || {
        let router: Router = Router::new();

        let new_service = move || {
            let router = router.clone();
            service_fn(move |req| nanoservices(req, &router))
        };

        let server = Server::bind(&addr)
            .serve(new_service)
            .map_err(|e| eprintln!("server error: {}", e));

        println!("Listening on http://{}", addr);
        server
    }));
}
