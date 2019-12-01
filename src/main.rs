#![deny(warnings)]

mod function;
mod router;
mod trigger;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Error, Method, Request, Response, Server};

use function::Function;
use router::Router;
use trigger::Trigger;

use futures_util::TryStreamExt;

use std::fs::create_dir_all;
use std::path::Path;

#[tokio::main]
async fn main() {
    create_dir_all(Path::new("data/")).unwrap();

    let addr = ([127, 0, 0, 1], 8000).into();

    let router = Router::new();

    let make_service = make_service_fn(move |_| {
        let router = router.clone();

        async move {
            Ok::<_, Error>(service_fn(move |req: Request<Body>| {
                let router = router.clone();

                async move {
                    Ok::<_, Error>(match (req.method(), req.uri().path()) {
                        (&Method::GET, "/function/") => {
                            let mut body = String::from("<h1>Functions</h1>");

                            router.select().into_iter().for_each(|v| {
                                body += format!("<a href=\"{}\">{}</a><br>", v.1.path(), v.1.name())
                                    .as_str()
                            });

                            Response::builder()
                                .status(200)
                                .header("Content-type", "text/html; charset=utf-8")
                                .body(body.into())
                                .unwrap()
                        }
                        (&Method::POST, "/function/") => {
                            let b = req.into_body().try_concat().await.unwrap().into_bytes();

                            match Function::from_json(&b).map(|f| f.build()) {
                                Some(Ok(f)) => {
                                    router.insert(f.trigger(), f);
                                    Response::new("Function Created".into())
                                }
                                Some(Err(e)) => {
                                    eprintln!("{}", e);
                                    Response::builder()
                                        .status(422)
                                        .body("Failed build process".into())
                                        .unwrap()
                                }
                                None => Response::builder()
                                    .status(422)
                                    .body("JSON error".into())
                                    .unwrap(),
                            }
                        }

                        (&Method::DELETE, "/function/") => {
                            let b = req.into_body().try_concat().await.unwrap().into_bytes();

                            match Function::from_json(&b) {
                                Some(f) => f.delete(router),
                                None => Response::builder()
                                    .status(422)
                                    .body("JSON error".into())
                                    .unwrap(),
                            }
                        }

                        (_, _) => {
                            let (parts, body) = req.into_parts();

                            match router.get(&Trigger::new(parts.method.as_str(), parts.uri.path()))
                            {
                                Some(f) => f.run(parts, body),
                                None => {
                                    Response::builder().status(404).body(Body::empty()).unwrap()
                                }
                            }
                        }
                    })
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);

    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
