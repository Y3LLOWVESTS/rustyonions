#!/usr/bin/env bash
# Scaffolds the ron-app-sdk-py directory and file layout.
# Run from repo root:
#   chmod +x scripts/scaffold_ron_app_sdk_py.sh
#   scripts/scaffold_ron_app_sdk_py.sh

set -euo pipefail

ROOT="sdk/ron-app-sdk-py"

echo "Scaffolding ron-app-sdk-py into: ${ROOT}"

# Helper: create directory if missing
mkd() {
  dir="$1"
  if [ ! -d "$dir" ]; then
    echo "  [dir]  $dir"
    mkdir -p "$dir"
  fi
}

# Helper: create file if missing (empty)
touch_if_missing() {
  file="$1"
  if [ ! -f "$file" ]; then
    echo "  [file] $file"
    touch "$file"
  fi
}

# Helper: create file with content only if missing
create_if_missing() {
  file="$1"
  shift
  if [ ! -f "$file" ]; then
    echo "  [file] $file"
    cat > "$file" <<'EOF'
EOF
    # now append provided content
    printf "%s\n" "$@" >> "$file"
  else
    echo "  [skip existing] $file"
  fi
}

###############################################################################
# Directories
###############################################################################

mkd "$ROOT"
mkd "$ROOT/tests"
mkd "$ROOT/tests/unit"
mkd "$ROOT/tests/integration"
mkd "$ROOT/tests/property"
mkd "$ROOT/tests/chaos"
mkd "$ROOT/examples"
mkd "$ROOT/docs"
mkd "$ROOT/ron_app_sdk_py"

###############################################################################
# Top-level files
###############################################################################

# Minimal pyproject stub (you will edit this later)
if [ ! -f "$ROOT/pyproject.toml" ]; then
  echo "  [file] $ROOT/pyproject.toml"
  cat > "$ROOT/pyproject.toml" <<'EOF'
[build-system]
requires = ["setuptools>=68", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "ron-app-sdk-py"
version = "0.1.0"
description = "Python App SDK for RON-CORE"
readme = "README.md"
requires-python = ">=3.11"
license = { text = "MIT OR Apache-2.0" }
authors = [
  { name = "Stevan White", email = "n/a" }
]

dependencies = [
  "httpx>=0.27.0",
  "pydantic>=2.8.0",
]

[project.optional-dependencies]
dev = [
  "pytest",
  "pytest-asyncio",
  "pytest-benchmark",
  "hypothesis",
  "tox",
  "ruff",
  "black",
  "mypy",
  "pip-audit",
]

[tool.setuptools.package-dir]
"" = "."

[tool.setuptools.packages.find]
where = ["."]
include = ["ron_app_sdk_py*"]
EOF
fi

# README stub (you already have the full content; this is a placeholder)
if [ ! -f "$ROOT/README.md" ]; then
  echo "  [file] $ROOT/README.md"
  cat > "$ROOT/README.md" <<'EOF'
# ron-app-sdk-py

Python App SDK for RON-CORE.

TODO: Replace this stub with the full README generated from the SDK README template.
EOF
fi

touch_if_missing "$ROOT/LICENSE-MIT"
touch_if_missing "$ROOT/LICENSE-APACHE"

if [ ! -f "$ROOT/CHANGELOG.md" ]; then
  cat > "$ROOT/CHANGELOG.md" <<'EOF'
# Changelog — ron-app-sdk-py

All notable changes to this project will be documented in this file.

## 0.1.0 (unreleased)
- Initial scaffold for ron-app-sdk-py.
EOF
fi

touch_if_missing "$ROOT/SDK_IDB.MD"
touch_if_missing "$ROOT/SDK_SECURITY.MD"
touch_if_missing "$ROOT/SDK_SCHEMA_IDB.MD"

# Basic root configs
if [ ! -f "$ROOT/.gitignore" ]; then
  cat > "$ROOT/.gitignore" <<'EOF'
__pycache__/
*.pyc
*.pyo
*.pyd
*.egg-info/
dist/
build/
.venv/
.mypy_cache/
.pytest_cache/
.cache/
EOF
fi

if [ ! -f "$ROOT/.editorconfig" ]; then
  cat > "$ROOT/.editorconfig" <<'EOF'
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
indent_style = space
indent_size = 4
trim_trailing_whitespace = true

[*.md]
max_line_length = off
EOF
fi

if [ ! -f "$ROOT/.ruff.toml" ]; then
  cat > "$ROOT/.ruff.toml" <<'EOF'
line-length = 100
target-version = "py311"

[lint]
select = ["E", "F", "W", "B"]
ignore = []

[format]
quote-style = "double"
EOF
fi

if [ ! -f "$ROOT/mypy.ini" ]; then
  cat > "$ROOT/mypy.ini" <<'EOF'
[mypy]
python_version = 3.11
warn_unused_configs = True
disallow_untyped_defs = True
disallow_incomplete_defs = True
check_untyped_defs = True
no_implicit_optional = True
warn_return_any = True
warn_unused_ignores = True
EOF
fi

###############################################################################
# tests/
###############################################################################

touch_if_missing "$ROOT/tests/__init__.py"

# unit tests
touch_if_missing "$ROOT/tests/unit/__init__.py"
touch_if_missing "$ROOT/tests/unit/test_client_basic.py"
touch_if_missing "$ROOT/tests/unit/test_errors_problem.py"
touch_if_missing "$ROOT/tests/unit/test_config_env.py"
touch_if_missing "$ROOT/tests/unit/test_pagination.py"
touch_if_missing "$ROOT/tests/unit/test_security_headers.py"

# integration tests
touch_if_missing "$ROOT/tests/integration/__init__.py"
touch_if_missing "$ROOT/tests/integration/test_healthz_readyz.py"
touch_if_missing "$ROOT/tests/integration/test_app_hello_roundtrip.py"
touch_if_missing "$ROOT/tests/integration/test_pagination_roundtrip.py"

# property tests
touch_if_missing "$ROOT/tests/property/__init__.py"
touch_if_missing "$ROOT/tests/property/test_idempotency_keys_hypothesis.py"
touch_if_missing "$ROOT/tests/property/test_error_parsing_fuzz.py"

# chaos tests
touch_if_missing "$ROOT/tests/chaos/__init__.py"
touch_if_missing "$ROOT/tests/chaos/test_timeouts_and_retries_toxiproxy.py"
touch_if_missing "$ROOT/tests/chaos/test_streaming_disconnects.py"

###############################################################################
# examples/
###############################################################################

touch_if_missing "$ROOT/examples/__init__.py"

if [ ! -f "$ROOT/examples/hello_async.py" ]; then
  cat > "$ROOT/examples/hello_async.py" <<'EOF'
import asyncio

from ron_app_sdk_py import RonClient, RonProblemError


async def main() -> None:
  client = RonClient.from_env()
  try:
    resp = await client.get("/app/hello")
    print("Hello (async):", resp)
  except RonProblemError as e:
    print("Problem:", e.problem.code, e.problem.message)
  finally:
    await client.aclose()


if __name__ == "__main__":
  asyncio.run(main())
EOF
fi

if [ ! -f "$ROOT/examples/hello_sync.py" ]; then
  cat > "$ROOT/examples/hello_sync.py" <<'EOF'
from ron_app_sdk_py import RonClientSync, RonProblemError


def main() -> None:
  client = RonClientSync.from_env()
  try:
    resp = client.get("/app/hello")
    print("Hello (sync):", resp)
  except RonProblemError as e:
    print("Problem:", e.problem.code, e.problem.message)
  finally:
    client.close()


if __name__ == "__main__":
  main()
EOF
fi

if [ ! -f "$ROOT/examples/list_paginated_items.py" ]; then
  cat > "$ROOT/examples/list_paginated_items.py" <<'EOF'
import asyncio

from ron_app_sdk_py import RonClient, iter_pages


async def main() -> None:
  client = RonClient.from_env()
  async for page in iter_pages(client, "/app/items", page_size=100):
    for item in page.items:
      print(item)
  await client.aclose()


if __name__ == "__main__":
  asyncio.run(main())
EOF
fi

if [ ! -f "$ROOT/examples/watch_events_async.py" ]; then
  cat > "$ROOT/examples/watch_events_async.py" <<'EOF'
import asyncio

from ron_app_sdk_py import RonClient


async def main() -> None:
  client = RonClient.from_env()
  async for event in client.subscribe("/app/events"):
    print("Event:", event)
  await client.aclose()


if __name__ == "__main__":
  asyncio.run(main())
EOF
fi

###############################################################################
# docs/
###############################################################################

if [ ! -f "$ROOT/docs/arch.mmd" ]; then
  cat > "$ROOT/docs/arch.mmd" <<'EOF'
flowchart LR
  subgraph Python App
    A[Caller: Python service / facet / CLI] -->|SDK calls| B(ron-app-sdk-py)
  end

  subgraph RON-CORE Node
    C[svc-gateway /app/*] --> D[omnigate /v1/app/*]
    D --> E[Micronode/Macronode]
  end

  B -->|HTTPS + caps| C
  E -->|metrics| F[[Prometheus]]

  style B fill:#0b7285,stroke:#083344,color:#fff
EOF
fi

if [ ! -f "$ROOT/docs/sequence_call.mmd" ]; then
  cat > "$ROOT/docs/sequence_call.mmd" <<'EOF'
sequenceDiagram
  actor PyApp as Python App
  participant C as RonClient
  participant G as svc-gateway
  participant O as omnigate
  participant N as Node

  PyApp->>C: get("/app/hello", token)
  C->>G: HTTPS GET /app/hello (Authorization: Bearer cap:...)
  G->>O: /v1/app/hello
  O->>N: internal route
  N-->>O: 200 OK (JSON)
  O-->>G: 200 OK
  G-->>C: 200 OK
  C-->>PyApp: parsed JSON or Problem error
EOF
fi

if [ ! -f "$ROOT/docs/state_client.mmd" ]; then
  cat > "$ROOT/docs/state_client.mmd" <<'EOF'
stateDiagram-v2
  [*] --> Unconfigured
  Unconfigured --> Configured: RonClient(config)
  Configured --> Ready: first request ok
  Ready --> Backoff: timeout / transient failure
  Backoff --> Ready: retry successful
  Ready --> Closed: aclose()/close()
  Backoff --> Closed: fatal error / shutdown
  Closed --> [*]
EOF
fi

touch_if_missing "$ROOT/docs/arch.svg"
touch_if_missing "$ROOT/docs/sequence_call.svg"
touch_if_missing "$ROOT/docs/state_client.svg"

###############################################################################
# ron_app_sdk_py/ package
###############################################################################

if [ ! -f "$ROOT/ron_app_sdk_py/__init__.py" ]; then
  cat > "$ROOT/ron_app_sdk_py/__init__.py" <<'EOF'
"""
ron-app-sdk-py

Python App SDK for RON-CORE.
"""

from ._version import __version__  # noqa: F401
from .config import ClientConfig  # noqa: F401
from .client import RonClient  # noqa: F401
from .client_sync import RonClientSync  # noqa: F401
from .errors import Problem, RonProblemError  # noqa: F401

__all__ = [
    "__version__",
    "ClientConfig",
    "RonClient",
    "RonClientSync",
    "Problem",
    "RonProblemError",
]
EOF
fi

if [ ! -f "$ROOT/ron_app_sdk_py/_version.py" ]; then
  cat > "$ROOT/ron_app_sdk_py/_version.py" <<'EOF'
__all__ = ["__version__"]

__version__ = "0.1.0"
EOF
fi

touch_if_missing "$ROOT/ron_app_sdk_py/config.py"
touch_if_missing "$ROOT/ron_app_sdk_py/client.py"
touch_if_missing "$ROOT/ron_app_sdk_py/client_sync.py"
touch_if_missing "$ROOT/ron_app_sdk_py/errors.py"
touch_if_missing "$ROOT/ron_app_sdk_py/models.py"
touch_if_missing "$ROOT/ron_app_sdk_py/pagination.py"
touch_if_missing "$ROOT/ron_app_sdk_py/streaming.py"
touch_if_missing "$ROOT/ron_app_sdk_py/facets.py"
touch_if_missing "$ROOT/ron_app_sdk_py/logging_.py"
touch_if_missing "$ROOT/ron_app_sdk_py/metrics.py"
touch_if_missing "$ROOT/ron_app_sdk_py/codecs.py"
touch_if_missing "$ROOT/ron_app_sdk_py/_types.py"

echo "Done. ron-app-sdk-py scaffold created."
