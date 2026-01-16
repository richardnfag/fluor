#[allow(warnings)]
mod bindings;

use bindings::Guest;

struct Component;

impl Guest for Component {
    /// Say hello!
    fn handle(request: String) -> String {
        format!("{}", request)
    }
}

bindings::export!(Component with_types_in bindings);
