from __future__ import annotations

from ron_app_sdk_py import Problem, RonProblemError


def test_problem_accepts_extra_fields_and_details_default() -> None:
    raw = {
        "code": "internal_error",
        "kind": "internal",
        "message": "Something went boom",
        "correlation_id": "abc123",
        "details": {"foo": "bar"},
        "extra_field": "ignored",
    }

    problem = Problem.model_validate(raw)

    assert problem.code == "internal_error"
    assert problem.kind == "internal"
    assert problem.message == "Something went boom"
    assert problem.correlation_id == "abc123"
    assert problem.details == {"foo": "bar"}
    # The extra field should not raise or appear on the model
    assert not hasattr(problem, "extra_field")


def test_problem_details_default_factory() -> None:
    problem = Problem(code="x", kind="y", message="z")
    assert problem.details == {}


def test_ron_problem_error_string_includes_code_and_message() -> None:
    problem = Problem(
        code="rate_limit",
        kind="client",
        message="Too many requests",
        correlation_id="req-1",
        details={"limit": 10},
    )
    err = RonProblemError(problem, status_code=429)

    s = str(err)
    assert "rate_limit" in s
    assert "Too many requests" in s
