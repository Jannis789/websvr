
use rama::http::service::web::WebService;
use crate::pages::home::HomePage;

pub fn router() -> WebService<()> {
	HomePage::mount(WebService::default())
}
