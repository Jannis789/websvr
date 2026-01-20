
use rama::http::service::web::WebService;
use rama::Context;

pub fn router() -> WebService<()> {
	WebService::default().get("/", |_: Context<()>| async { "Hello, Rama!" })
                         .get("/test", |_: Context<()>| async { "This is a test endpoint." })
}
