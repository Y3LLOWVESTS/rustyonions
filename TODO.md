# TODO
---

## 1) Lock the “app contract” in ron-app-sdk (Rust)

Before we ship a TS SDK, we need the core protocol frozen:

* Define the **minimal “Hello world app” contract**:

  * Auth model (what token/cookie/header a request carries).
  * How an app declares routes / facets (even if it’s a placeholder).
  * How gateway/omnigate forwards to app code.
* Make sure **ron-app-sdk (Rust)** exposes that clearly:

  * Types for requests/responses, error envelopes.
  * A tiny server helper (“mount this app on a micronode/macronode gateway”).
* Add a couple of concrete integration tests:

  * `micronode + simple app facet + gateway` round-trip.
  * `macronode + app` round-trip.

This lets us say: **“here is the contract all SDKs must follow”**.

---

## 2) Wire macronode/micronode ↔ gateway/omnigate ↔ app

Very thin slice:

* Make **gateway** expose an `/app` (or `/api`) prefix for app traffic.
* Use **omnigate** (or a simple subset) to route:

  * `/app/…` → app handler via ron-app-sdk.
* One example app wired in-tree (even just “echo” / “kv put/get”).

At that point, we have a **real end-to-end app path**.

---

## 3) Build **ron-app-sdk-ts**

Now that the contract is stable, build the TS SDK to speak it:

* New crate/repo: `ron-app-sdk-ts`.
* Minimal features first:

  * Config: base URL, auth token.
  * Helpers: `sdk.get("/foo")`, `sdk.post("/bar", body)`.
  * Convenience wrappers for any “blessed” app patterns (e.g. KV calls, jobs).
* Test it against a local micronode/macronode running the sample app.

This is where external JS/TS devs can start actually hacking.

---

## 4) Create **svc-admin** (GUI + admin API)

Now layer the operator UX on top of what we already have:

* New crate: `svc-admin` (HTTP + static assets).
* UI talks to existing admin plane endpoints:

  * `/healthz`, `/readyz`, `/metrics`, `/api/v1/status`.
* Initial screens:

  * Node overview (status, deps, services, uptime, log level).
  * Service list with live status.
  * Maybe a very simple “apps” page showing installed facets/apps once the app plane can enumerate them.

`svc-admin` can be served:

* Either as its own service behind gateway, or
* Mounted as a static bundle off macronode’s admin plane; we can pick later.

---

## 5) After that

Once the app path and GUI are real:

* Tighten readiness to eventually include overlay/mailbox/dht once those crates are non-stub.
* Add TLS + admin auth (passports/macaroon) around `/api/v1/*` and `/admin` GUI.
* Iterate the SDKs (Rust + TS) with nicer ergonomics.

So: **yes**, we’re basically at the point where the next big push is the **app SDK + TS SDK + svc-admin GUI**, with the only caveat being: freeze the server-side app contract in `ron-app-sdk` and gateway/omnigate *before* we finalize `ron-app-sdk-ts`.
