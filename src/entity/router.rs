
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::entity::function::Function;
use crate::entity::trigger::Trigger;


#[derive(Clone)]
pub struct Router {
    router: Arc<Mutex<HashMap<Trigger, Function>>>,
}