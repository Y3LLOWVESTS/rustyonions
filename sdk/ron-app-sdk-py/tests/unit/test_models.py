from __future__ import annotations

from ron_app_sdk_py.models import Event, HelloResponse, HealthzResponse, ReadyzResponse


def test_hello_response_model() -> None:
    obj = HelloResponse(message="hi")
    assert obj.message == "hi"


def test_healthz_readyz_models() -> None:
    h = HealthzResponse(ok=True, service="app")
    assert h.ok is True
    assert h.service == "app"

    r = ReadyzResponse(ready=False, reason="warming up")
    assert r.ready is False
    assert r.reason == "warming up"


def test_event_json_data_and_dict_data() -> None:
    e = Event(id="1", event="update", data='{"foo": 1, "bar": "x"}')
    parsed = e.json_data()
    assert isinstance(parsed, dict)
    assert parsed["foo"] == 1
    assert e.dict_data == {"foo": 1, "bar": "x"}

    # Non-JSON payload falls back to raw string / empty dict.
    e2 = Event(data="not-json")
    assert e2.json_data() == "not-json"
    assert e2.dict_data == {}
