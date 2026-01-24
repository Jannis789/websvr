use rama::http::service::web::WebService;
use crate::pages::home::HomePage;
use crate::util::public_service::PublicService;

pub fn router() -> WebService<()> {
    let svc = WebService::default();
    let svc = PublicService::mount(svc); // mount gibt den neuen svc zurück
    let svc = HomePage::mount(svc);       // mount gibt wieder zurück
    svc
}