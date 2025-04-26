use exports::wassel::plugin::http_plugin::{self, Guest, GuestHandler};
use wasi::http::types::{Headers, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam};

wit_bindgen::generate!({
    world: "plugin",
    generate_all,
});

struct Plugin;

impl Guest for Plugin {
    type Handler = HttpHandler;

    fn instantiate() -> http_plugin::Plugin {
        http_plugin::Plugin {
            name: "hello-plugin".to_owned(),
            version: "0.0.0".to_owned(),
            description: None,
            endpoints: vec![
                http_plugin::Endpoint {
                    path: "/hello".to_owned(),
                    handler: http_plugin::Handler::new(HttpHandler {
                        func: Box::new(handle_hello),
                    }),
                },
                http_plugin::Endpoint {
                    path: "/bye".to_owned(),
                    handler: http_plugin::Handler::new(HttpHandler {
                        func: Box::new(handle_bye),
                    }),
                },
            ],
        }
    }
}

pub struct HttpHandler {
    func: Box<dyn Fn() -> String>,
}

impl GuestHandler for HttpHandler {
    fn handle(&self, _req: IncomingRequest, out: ResponseOutparam) {
        let res = OutgoingResponse::new(Headers::new());
        res.set_status_code(200).unwrap();
        let body = res.body().unwrap();
        let stream = body.write().unwrap();
        ResponseOutparam::set(out, Ok(res));
        stream.write((self.func)().as_bytes()).unwrap();
        drop(stream);
        OutgoingBody::finish(body, None).unwrap();
    }
}

fn handle_hello() -> String {
    "Hello from plugin\n".to_owned()
}

fn handle_bye() -> String {
    "Goodbye from plugin\n".to_owned()
}

export!(Plugin);
