#[allow(warnings)]
mod bindings;

use bindings::Guest;

struct Component;

impl Guest for Component {
    fn handle(_input: String) -> String {
        r#"{"status": "ok"}"#.to_string()
    }
}

bindings::export!(Component with_types_in bindings);
