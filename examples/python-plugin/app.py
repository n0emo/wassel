from typing import override

from plugin import exports
from plugin.exports import http_plugin
from plugin.imports import types
from plugin.types import Ok

class PythonHandler(http_plugin.Handler):
    @override
    def handle(self, request: types.IncomingRequest, out: types.ResponseOutparam) -> None:
        res = types.OutgoingResponse(types.Fields())
        res.set_status_code(200)
        body = res.body()
        types.ResponseOutparam.set(out, Ok(res))
        with body.write() as stream:
            stream.write(b"Hello from python")
        types.OutgoingBody.finish(body, None)

class HttpPlugin(exports.HttpPlugin):
    @override
    def instantiate(self) -> http_plugin.Plugin:
        return exports.http_plugin.Plugin(
            name="python-plugin",
            version="0.0.0",
            description=None,
            endpoints=[
                http_plugin.Endpoint(
                    path="/python",
                    handler=PythonHandler(),
                )
            ],
        )

