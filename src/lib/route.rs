use crate::lib::{HttpRequest, HttpResponse};

#[derive(Debug, Clone)]
pub struct Route {
    pub method: String,
    pub path: String,
    pub handler: fn(&HttpRequest) -> HttpResponse,
}
