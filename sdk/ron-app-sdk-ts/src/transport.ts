/**
 * Transport layer: wraps fetch() with URL construction, timeouts, and
 * response parsing.
 */

import type { AppRequest, AppResponse, RonOptions } from './types';
import {
  applyObservabilityHeaders,
  buildHeaders,
  generateRequestId,
} from './utils/headers';
import { withTimeout } from './utils/timeouts';
import { joinBaseAndPath, toQueryString } from './utils/url';
import { parseProblem } from './utils/problem';

/**
 * Core HTTP transport.
 *
 * Responsibilities:
 * - Build `<baseUrl>/app<path>` and append query string.
 * - Merge user headers + headerProvider + auth/passport headers.
 * - Attach x-request-id / x-correlation-id.
 * - Apply overall timeout via AbortController.
 * - Map:
 *   - 2xx JSON → data
 *   - 4xx/5xx JSON → RonProblem
 *   - non-JSON / malformed / local errors → safe RonProblem with kind "transport".
 *
 * Invariants:
 * - Never leaks authToken/passportToken into URLs or error messages.
 * - Never throws for server-originated problems; local network/timeout are
 *   represented as AppResponse with status = 0 and problem.kind = "transport".
 */
export async function sendRequest<T = unknown>(
  options: RonOptions,
  req: AppRequest,
): Promise<AppResponse<T>> {
  const {
    baseUrl,
    overallTimeoutMs = 10_000,
    requestIdFactory,
    headerProvider,
    onRequest,
    onResponse,
    authToken,
    passportToken,
  } = options;

  // 1) Build URL (never embed secrets into URL or query).
  const base = baseUrl;
  const urlPath = joinBaseAndPath(base, req.path);
  const qs = toQueryString(req.query);
  const url = `${urlPath}${qs ?? ''}`;

  // 2) Header provider (for token rotation, volatile caps, etc.).
  let extraHeaders: Record<string, string> = {};
  if (headerProvider) {
    const provided = await headerProvider();
    if (provided) {
      extraHeaders = provided;
    }
  }

  // 3) Observability headers (request/correlation IDs).
  const idFactory = requestIdFactory ?? generateRequestId;
  let effectiveReq: AppRequest = applyObservabilityHeaders(
    { ...req },
    idFactory,
  );

  // 4) Base headers: user + headerProvider.
  let headers = buildHeaders(effectiveReq.headers, extraHeaders);

  // 5) Auth / capabilities — headers only, never URL.
  if (authToken) {
    headers = buildHeaders(headers, {
      authorization: authToken,
    });
  }
  if (passportToken) {
    headers = buildHeaders(headers, {
      'x-ron-passport': passportToken,
    });
  }

  // 6) Content negotiation.
  headers = buildHeaders(headers, {
    accept: 'application/json',
  });

  // 7) Body handling and content-type.
  let body: BodyInit | undefined;
  if (
    effectiveReq.body !== undefined &&
    effectiveReq.body !== null &&
    effectiveReq.method !== 'GET'
  ) {
    const bodyValue = effectiveReq.body;

    if (typeof FormData !== 'undefined' && bodyValue instanceof FormData) {
      body = bodyValue;
      // browser will set content-type with boundary.
    } else if (typeof Blob !== 'undefined' && bodyValue instanceof Blob) {
      body = bodyValue;
    } else if (
      typeof ArrayBuffer !== 'undefined' &&
      bodyValue instanceof ArrayBuffer
    ) {
      body = bodyValue;
    } else {
      headers = buildHeaders(headers, {
        'content-type': 'application/json',
      });
      body = JSON.stringify(bodyValue);
    }
  }

  effectiveReq = { ...effectiveReq, headers };

  // 8) Optional onRequest hook (DX / debugging).
  if (onRequest) {
    try {
      onRequest({ ...effectiveReq, url });
    } catch {
      // Swallow hook errors; never break app calls.
    }
  }

  const { signal, cancel } = withTimeout(overallTimeoutMs);

  let appRes: AppResponse<T>;

  try {
    const fetchFn: typeof fetch =
      typeof fetch !== 'undefined'
        ? fetch
        : ((() => {
            throw new Error('global fetch is not available');
          }) as unknown as typeof fetch);

    const res = await fetchFn(url, {
      method: effectiveReq.method,
      headers,
      body,
      signal,
    });

    const headerObj: Record<string, string> = {};
    res.headers.forEach((value, key) => {
      headerObj[key.toLowerCase()] = value;
    });

    if (res.ok) {
      const respContentType = res.headers.get('content-type') ?? '';
      let data: T | undefined;
      let raw: ArrayBuffer | undefined;

      if (respContentType.includes('application/json')) {
        try {
          const json = (await res.json()) as unknown as T;
          data = json;
        } catch {
          const problem = await parseProblem(res);
          appRes = {
            status: res.status,
            ok: false,
            headers: headerObj,
            problem,
          };
          finalizeResponse(onResponse, appRes);
          return appRes;
        }
      } else {
        raw = await res.arrayBuffer();
      }

      appRes = {
        status: res.status,
        ok: true,
        headers: headerObj,
        data,
        raw,
      };
    } else {
      const problem = await parseProblem(res);
      appRes = {
        status: res.status,
        ok: false,
        headers: headerObj,
        problem,
      };
    }
  } catch (err) {
    // Local network / timeout / AbortError.
    const isAbortError =
      err &&
      typeof err === 'object' &&
      (err as { name?: string }).name === 'AbortError';

    const code = isAbortError ? 'local_timeout' : 'local_network_failure';
    const message = isAbortError ? 'Request timed out' : 'Network error';

    // Do NOT leak err.message (could theoretically contain tokens).
    appRes = {
      status: 0,
      ok: false,
      headers: {},
      problem: {
        code,
        message,
        kind: 'transport',
        retryable: true,
      },
    };
  } finally {
    cancel();
  }

  finalizeResponse(onResponse, appRes);
  return appRes;
}

function finalizeResponse<T>(
  hook: RonOptions['onResponse'] | undefined,
  res: AppResponse<T>,
): void {
  if (!hook) return;
  try {
    hook(res);
  } catch {
    // Hooks are best-effort only.
  }
}
