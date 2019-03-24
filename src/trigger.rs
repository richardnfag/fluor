#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct Trigger {
    method: String,
    path: String,
}

impl Trigger {
    pub fn new(method: &str, path: &str) -> Trigger {
        Trigger {
            method: method.to_string(),
            path: path.to_string(),
        }
    }
}
