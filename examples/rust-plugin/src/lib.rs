use wassel_sdk_rust::bindings::{
    export,
    exports::wassel::foundation::http_handler::Guest,
    wasi::{
        self,
        http::types::{Headers, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam},
    },
};

struct Plugin;

impl Guest for Plugin {
    fn handle_request(request: IncomingRequest, response_out: ResponseOutparam) -> () {
        handle(request, response_out, "Goodbye from Rust");
    }
}

fn handle(_req: IncomingRequest, out: ResponseOutparam, text: &str) {
    let base_url = wasi::config::store::get("base_url")
        .ok()
        .flatten()
        .unwrap_or_else(|| "No base url".to_owned());

    let res = OutgoingResponse::new(Headers::new());
    res.set_status_code(200).unwrap();
    let body = res.body().unwrap();
    let stream = body.write().unwrap();
    ResponseOutparam::set(out, Ok(res));
    let response = format!("{}\n{}\n", text, base_url,);
    stream.write(response.as_bytes()).unwrap();
    drop(stream);
    OutgoingBody::finish(body, None).unwrap();
}

export!(Plugin);
