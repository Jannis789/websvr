mod counter {
    use rama::http::Response;
    use std::sync::atomic::{AtomicI32, Ordering};

    static COUNTER: AtomicI32 = AtomicI32::new(0);

    fn increment() -> i32 {
        COUNTER.fetch_add(1, Ordering::Relaxed) + 1
    }

    // GET → Patch Signals (JSON)
    pub fn increment_response() -> Response {
        let value = increment();

        Response::builder()
            .header("content-type", "application/json")
            .body(format!(r#"{{ "counter": {} }}"#, value).into())
            .unwrap()
    }

    // HTML nur für SSE
    pub fn get_template() -> &'static str {
        r#"
            <div data-signals:counter="0">
	            <button data-on:click="@get('/increment')">
		                Increment
	            </button>
	            <div data-text="$counter"></div>
            </div>
		"#
    }
}
