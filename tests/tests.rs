extern crate fluor;

use fluor::function::Function;
use futures::{Future, Stream};
use hyper::Request;

use std::fs::create_dir_all;
use std::path::Path;

static FUNCTION : &'static [u8] = br#"{
    "name": "hello-rust",
    "language": "rust",
    "source": "H4sIANNokVwAA+2VTUvEMBCGe86vmM1pF6QmadoKfiAI4smD12WR0MZu2DZZklQP4n83xbLIgu7Froh5LhMmHzOZl0mcrU6TiSGElGUOwdIyJ6Olg90BNOOcs4KzMCaUlZwmkE+d2EDvvLAhFauqtbD1l+sOzY/32Nk/ggv6d0Lp1LrJYoR6FAX/Rv8839OfsyxLgEyW0Sf+uf5PGgb55wt4RRDYWqV9q2dzfCfb1pzAi7FtPcOLc/SGfjvZyI9zI2xjUm+6droYh/qfsWy//8OHEfv/GCy3otqIRq6QFp2ES8Droe8xepbWKaMHD0lpSjASvV8b64JniR8+qgG1gXvhKtVJ7Q3ciqbXtXRwMVZLC/d4bXrfGrNJK9Nd4RWStfLjwYzQM4zQspZbGfbpSkm3iq9MJBKJHIN3ylDQlgAOAAA",
    "method": "GET",
    "path": "/hello-rust/",
    "cpu": "2",
    "memory": "1024m",
    "uptime": "30"
}"#;

#[test]
fn create_function() {
    let f = Function::from_json(FUNCTION);
    assert_eq!(f.is_some(), true);
}

#[test]
fn build_function() {
    create_dir_all(Path::new("data/")).unwrap();
    let f = Function::from_json(FUNCTION).unwrap().build();
    assert_eq!(f.is_ok(), true);
}

#[test]
fn run_function() {
    let f = Function::from_json(FUNCTION).unwrap().build().unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("https://localhost:8000")
        .body("".into())
        .unwrap();

    let (parts, body) = request.into_parts();

    let res = f.run(parts, body);

    let res = res
        .into_body()
        .concat2()
        .wait()
        .ok()
        .map(|b| b.into_bytes());

    assert_eq!(&res.unwrap()[..], b"Hello, world!\n");
}
