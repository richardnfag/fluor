use futures_util::TryStreamExt;
use hyper::{Body, Client, Request};
use tokio::runtime::Runtime;

#[test]
fn function() {
    let rt = Runtime::new().unwrap();

    rt.block_on(create_function());
    rt.block_on(run_function());
    rt.block_on(delete_function());
}

async fn create_function() {
    let body = r#"{
        "name": "hello",
        "language": "rust",
        "source": "H4sIAAAAAAAAA+3VTUvDMBgH8J7zKbKcNpAu6avgC4Ignjx4HUNCm21hbTKS1ov43W3dhCHoQOiG7P+7BJK0ecgfnoTTYHC8k+dpP4o85fvjl0DEaRbzhCd5FnAheJwHNB2+tCBofSMdpYHTxUq68sd9h9b/qXB6L93Sho2tq6HO6APOsuTH/KMo/pZ/lHfbKR+qoH1nnv9sI4u1XKo5MbJW9Iaylaoqy8ircl5b08/wUIScEdk2K+t8NzNjz9vboKWlT9IXulamsfRBLltTKk+vd7dlpH+5s21TWbsOC1vfsjlRpW52P464uGSEzEq1Ud13ptDKz8mpr+SshFPvioHfgD/0/yzK0P+PYZt/LbUJnR/ojEP9n6fiM/80E0ke9fnHonsS0P+PYGFoH/54Qt8I7WycNk1lRmP2qC/oomqtG7HJFXk/daEAAAAAAAAAAAAAAAAAAPCrD2TfwBsAKAAA",
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

    let res = client
        .request(req)
        .await
        .unwrap()
        .into_body()
        .try_concat()
        .await
        .expect("request body concat")
        .into_bytes();

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

    let res = client
        .request(req)
        .await
        .unwrap()
        .into_body()
        .try_concat()
        .await
        .expect("request body concat")
        .into_bytes();

    assert_eq!(String::from_utf8_lossy(&res), String::from("Hello World!\n"));
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

    let res = client
        .request(req)
        .await
        .unwrap()
        .into_body()
        .try_concat()
        .await
        .expect("request body concat")
        .into_bytes();

    assert_eq!(
        String::from_utf8_lossy(&res),
        String::from("Function Deleted")
    );
}
