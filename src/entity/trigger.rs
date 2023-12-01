#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct Trigger {
    method: Method,
    resource: String,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
enum Method {
    HttpPost,
    HTTP_GET,
    HTTP_PUT,
    HTTP_DELETE,
    SCHEDULED,
}

fn main() {
    let trigger: Trigger = Trigger{
        method: Method::HTTP_GET,
        resource: "/v1/function".to_owned()
    };
    let trigger: Trigger = Trigger{
        method: Method::HTTP_GET,
        resource: "/v1/function".to_owned()
    };
}