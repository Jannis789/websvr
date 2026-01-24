use rama::http::service::web::WebService;
use crate::pages::home::HomePage;
use crate::util::public_service::PublicService;

pub fn router() -> WebService<()> {
    let svc = WebService::default();
    let svc = PublicService::serve(svc);
    let svc = HomePage::serve(svc);
    svc
}