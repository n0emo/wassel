use exports::wassel::plugin::http_plugin::{self, Guest, GuestHandler};

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
    func: Box<dyn Fn() -> String>
}

impl GuestHandler for HttpHandler {
    fn handle(&self) -> String {
        (self.func)()
    }
}

fn handle_hello() -> String {
    "Hello from plugin\n".to_owned()
}

fn handle_bye() -> String {
    "Goodbye from plugin\n".to_owned()
}

export!(Plugin);
