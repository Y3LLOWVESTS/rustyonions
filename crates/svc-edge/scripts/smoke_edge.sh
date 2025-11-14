#!/usr/bin/env bash
set -euo pipefail

ADMIN=${ADMIN:-127.0.0.1:9909}
API=${API:-127.0.0.1:8080}
ASSETS_DIR=${SVC_EDGE_ASSETS_DIR:-assets}
TEST_FILE=${TEST_FILE:-hello.txt}

echo "[INFO] Admin plane checks @ http://${ADMIN}"
curl -fsS -o /dev/null http://${ADMIN}/healthz && echo "[OK] /healthz"
curl -fsS -o /dev/null http://${ADMIN}/metrics && echo "[OK] /metrics"

echo "[STEP] Ensure assets dir + test file"
mkdir -p "${ASSETS_DIR}"
if [ ! -f "${ASSETS_DIR}/${TEST_FILE}" ]; then
  echo "hello-edge" > "${ASSETS_DIR}/${TEST_FILE}"
fi

echo "[STEP] Readiness"
curl -fsS http://${ADMIN}/readyz && echo

echo "[STEP] GET /edge/assets/${TEST_FILE}"
resp_headers=$(mktemp)
curl -i -fsS http://${API}/edge/assets/${TEST_FILE} | tee "${resp_headers}" | sed -n '1,999p' >/dev/null
etag=$(grep -i '^etag:' "${resp_headers}" | awk '{print $2}' | tr -d '\r')
echo "[OK] ETag: ${etag}"

echo "[STEP] 304 via If-None-Match"
curl -i -fsS http://${API}/edge/assets/${TEST_FILE} -H "If-None-Match: ${etag}" | head -n 1

echo "[STEP] 206 via Range: bytes=2-"
curl -i -fsS http://${API}/edge/assets/${TEST_FILE} -H 'Range: bytes=2-' | sed -n '1,6p'

echo "[STEP] CAS demo (blake3)"
# Use the same ETag (it’s the blake3 digest) to stage CAS file
digest=$(echo "${etag}" | tr -d '"')
mkdir -p "${ASSETS_DIR}/cas/blake3"
cp "${ASSETS_DIR}/${TEST_FILE}" "${ASSETS_DIR}/cas/blake3/${digest}"
curl -i -fsS http://${API}/cas/blake3/${digest} | sed -n '1,6p'
curl -i -fsS http://${API}/cas/blake3/${digest} -H 'Range: bytes=2-' | sed -n '1,6p'

echo "[STEP] Burst /edge/assets/${TEST_FILE} (200×200)"
seq 1 200 | xargs -n1 -P200 -I{} curl -s -o /dev/null -w "%{http_code}\n" \
  http://${API}/edge/assets/${TEST_FILE} | sort | uniq -c

echo "[DONE] smoke_edge.sh completed successfully."
