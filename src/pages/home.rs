
// use async_stream::stream;
// use rama::http::Response;
use rama::http::service::web::WebService;
use crate::util::into_html_response::into_html_response;
// use crate::util::datastar::patch_element;
// use crate::components::header;
// use crate::components::body;
// use crate::components::footer;

pub struct HomePage;

impl HomePage {
	pub fn mount(svc: WebService<()>) -> WebService<()> {
        svc.with_get("/", into_html_response(HomePage::get_layout_template()))
		// svc.set_get("/patch-home", |_, _| Self::patch_stream());
		// svc.set_get("/increment", |_, _| counter::increment_response());
	}

	// STRING ONLY (wie gefordert)
	fn get_layout_template() -> &'static str {
		r#"
            <!doctype html>
            <html>
            <head>
            	<meta charset="utf-8">
            	<title>Home</title>
                
            	<script
            		type="module"
            		src="https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.0-RC.7/bundles/datastar.js">
            	</script>
            </head>
            <body>
            	<div id="header"></div>
            	<div id="body"></div>
            	<div id="footer"></div>
            </body>
            </html>
		"#
	}

	// // SSE über normalen GET
	// fn patch_stream() -> Response {
	// 	let stream = stream! {
	// 		yield patch_element("header", header::get_template());
	// 		yield patch_element("body", body::get_template());
	// 		yield patch_element("body-content", counter::get_template());
	// 		yield patch_element("footer", footer::get_template());
	// 	};
// 
	// 	Response::builder()
	// 		.header("content-type", "text/event-stream")
	// 		.body(stream.into())
	// 		.unwrap()
	// }
}