use hyper::{Body, Client, Request};
use tokio::runtime::Runtime;

#[test]
fn function() {
    let mut rt = Runtime::new().unwrap();

    rt.block_on(create_function());
    rt.block_on(run_function());
    rt.block_on(delete_function());
}

async fn create_function() {
    let body = r#"{
        "name": "hello",
        "language": "rust",
        "source": "H4sIAHFK5F0AA+3VTWvCMBgH8JzzKWJOeolJa9vBXhgMxk477LKDyAhtpsU2kaTdZey7z6IDEZwwqDL8/y6B9CUPzx+eiDHpnVzLsqRbVZbI3fUHUXGSKTmZpColUikZp4Ql/ZdGSBsa7RkjvswX2hcH3zv2/J8S4wft5040rq76OqMLOE0nB/OPongv/yhLY8JkXwXtuvD8pyudL/XczKjVtWG3jC9MVTlOP4wPpbPdjhRKSE512yycD+udKX/ZdIMVjj3rkJe1sY1jj3re2sIEdrPtltXh7d61TeXcUuSuvuMzaoqy2f44kuqKUzotzMqsv7N5acKMnrslF0WMg897vgP+MP/TKMH8P4VN/rUurfChpzOOzX+Z7M//WKkE8/8U3i3rwh+O2Cdlaytf2qaygyF/6u4B9up8VQz46Jp+nbtUAAAAAAAAAAAAAAAAAAD4xTf6PZ90ACgAAA==",
        "method": "GET",
        "path": "/hello/",
        "cpu": "2",
        "memory": "1024m",
        "uptime": "30"
    }"#;

    let client = Client::new();

    let req = Request::builder()
        .method("POST")
        .uri("http://127.0.0.1:8000/function/")
        .body(Body::from(body))
        .expect("request builder");

    let res = client.request(req).await.unwrap();

    let res = hyper::body::to_bytes(res.into_body()).await.unwrap();

    assert_eq!(
        String::from_utf8_lossy(&res),
        String::from("Function Created")
    );
}

async fn run_function() {
    let client = Client::new();

    let req = Request::builder()
        .method("GET")
        .uri("http://127.0.0.1:8000/hello/")
        .body(Body::from(""))
        .expect("request builder");

    let res = client.request(req).await.unwrap();

    let res = hyper::body::to_bytes(res.into_body()).await.unwrap();

    assert_eq!(
        String::from_utf8_lossy(&res),
        String::from("Hello World!\n")
    );
}

async fn delete_function() {
    let body = r#"{
        "name": "hello",
        "language": "rust",
        "source": "",
        "method": "GET",
        "path": "/hello/",
        "cpu": "2",
        "memory": "1024m",
        "uptime": "30"
    }"#;

    let client = Client::new();

    let req = Request::builder()
        .method("DELETE")
        .uri("http://127.0.0.1:8000/function/")
        .body(Body::from(body))
        .expect("request builder");

    let res = client.request(req).await.unwrap();

    let res = hyper::body::to_bytes(res.into_body()).await.unwrap();

    assert_eq!(
        String::from_utf8_lossy(&res),
        String::from("Function Deleted")
    );
}
