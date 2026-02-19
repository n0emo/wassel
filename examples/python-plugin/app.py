from typing import override
from wassel_sdk import http


class HttpHandler(http.HttpHandler):
    @override
    def handle(self, _req: http.Request) -> http.Response:
        return http.Response(body=bytes("Hello from my super plugin", "utf-8"))
