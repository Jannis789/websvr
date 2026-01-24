use rama::http::service::web::WebService;

pub struct PublicService;

impl PublicService {
    pub fn mount(svc: WebService<()>) -> WebService<()> {
        svc.with_dir("/public", "public")
    }
}
