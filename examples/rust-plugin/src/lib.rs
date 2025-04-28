use exports::wassel::plugin::http_plugin::{self, Endpoint};
use wasi::http::types::{Headers, OutgoingBody, OutgoingResponse};

wit_bindgen::generate!({
    world: "plugin",
    generate_all,
});

struct Plugin;

impl Guest for Plugin {
    fn handle_hello(request: IncomingRequest, response_out: ResponseOutparam) {
        handle(request, response_out, "Hello from Rust");
    }

    fn handle_bye(request:IncomingRequest,response_out:ResponseOutparam) {
        handle(request, response_out, "Goodbye from Rust");
    }
}

impl http_plugin::Guest for Plugin {
    fn get_endpoints() -> Vec::<Endpoint> {
        vec![
            Endpoint { path: "/hello".to_owned(), handler: "handle-hello".to_owned() },
            Endpoint { path: "/bye".to_owned(), handler: "handle-bye".to_owned() },
        ]
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
    let response = format!("{}\n{}\n",
        text,
        base_url,
    );
    stream.write(response.as_bytes()).unwrap();
    drop(stream);
    OutgoingBody::finish(body, None).unwrap();
}

export!(Plugin);
