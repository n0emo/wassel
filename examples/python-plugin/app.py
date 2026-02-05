import pathlib
from typing import override, List

import plugin
from plugin import exports
from plugin.exports.http_plugin import Endpoint
from plugin import Ok
from plugin.imports.wasi_http_types import (
    IncomingRequest,
    ResponseOutparam,
    OutgoingResponse,
    Fields,
    OutgoingBody,
)

from jinja2 import Environment, DictLoader, select_autoescape

def read_entire_file(path: str) -> str:
    final_path = pathlib.Path(__file__).parent.resolve().joinpath(path)
    with open(final_path, 'r') as file:
        return file.read()

templates = {
    "index.html": read_entire_file("templates/index.html")
}

env = Environment(
    loader=DictLoader(templates),
    autoescape=select_autoescape()
)

class HttpPlugin(exports.HttpPlugin):
    @override
    def get_endpoints(self) -> List[Endpoint]:
        return [Endpoint("/", "handle-root")]


class Plugin(plugin.Plugin):
    @override
    def handle_root(
        self,
        request: IncomingRequest,
        response_out: ResponseOutparam
    ) -> None:
        template = env.get_template("index.html");
        html = template.render().encode('utf-8')

        res = OutgoingResponse(Fields())
        res.set_status_code(200)
        body = res.body()
        ResponseOutparam.set(response_out, Ok(res))
        with body.write() as stream:
            stream.write(html)
        OutgoingBody.finish(body, None)
