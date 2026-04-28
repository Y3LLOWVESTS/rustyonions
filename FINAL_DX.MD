Yes — the final DX can absolutely be:

```bash
micronode run my-app -- ./my-app-binary --port 5401
```

…and the dev never touches your repo, never compiles RON-CORE, and never “puts facets inside a binary.”

The key is: **facets/manifests live *next to* the app artifact (or are generated), not inside it.** The binary stays immutable; the node reads external config.

## What the dev must provide (minimal)

They must provide **two things** (but we can make it feel like “one thing”):

1. **App artifact**: a binary path (or container later)
2. **Mount intent**: where it should be exposed (facet/proxy/static routes)

For God-tier DX, we hide #2 behind defaults or auto-generation.

## The God-tier way: micronode generates the facet for them

Instead of “specify app location *and* facet location,” do:

```bash
micronode run my-app --cmd ./my-app-binary -- --port 5401
```

Micronode then:

* picks an internal port (or uses `--port`)
* **auto-creates a proxy facet**:

  * facet id: `my-app`
  * mounted path: `/facets/my-app/*`
  * upstream: `http://127.0.0.1:<port>`
* writes it to a local runtime dir (not inside the binary), e.g.:

  * `~/.ron/micronode/apps/my-app/manifest.toml`
  * `~/.ron/micronode/apps/my-app/app.json` (or similar)
* starts supervising the process + exposes it.

So the dev only supplies *the binary path* (and optional args). Everything else is inferred.

## When would they ever specify facets manually?

Only when they want something non-default, like:

* custom public URL (`/facets/hello` vs `/facets/my-app`)
* multiple routes
* static assets + proxy combined
* auth requirements/policy intent per route
* multiple upstreams

Then they can pass a manifest explicitly:

```bash
micronode run --app ./my-app-binary --manifest ./ron.toml
```

…but that’s “advanced mode,” not the default.

## What this means for “download links / no compiling”

Totally compatible. Shipping as:

* `micronode` binary download
* `ronctl` (optional control CLI)
* `svc-admin` binary (optional)
* “hello-app template” repo or `ron init` scaffold

Devs bring *their* artifacts; micronode runs them and generates/loads the manifests externally.

## The contract we’re building toward

* **App = artifact**
* **Facet = declaration of how app is exposed**
* **Micronode = supervisor + router + policy + metrics**
* **Default path**: micronode can generate facets automatically from `micronode run ...`

---

### Where this ties back to what we’re doing right now

The next demo (hello-app + proxy facet) is literally proving the core primitive that makes this DX possible:

* “external artifact”
* “external manifest”
* “proxy mount”
* “observable control (reload/crash)”
* “no repo compilation required”

When we finish this demo, we’ll be one step away from implementing the UX sugar: `micronode run <name> -- <cmd...>` which just **writes the manifest for you**.

If you tell me what command shape you prefer (one of the two below), I’ll align the hello-app + script + manifest naming to match:

1. `micronode run hello -- ./hello-app`
2. `micronode -app hello --cmd ./hello-app`


######

