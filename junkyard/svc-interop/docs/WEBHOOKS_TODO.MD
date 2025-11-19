
>AI GENERATED CONTENT ON WHAT WEBHOOKS SUPPORT TO INCLUDE IN PROJECT:
>PLEASE NOTE THAT THIS LIST IS NOT EXHAUSTIVE

Here’s a paste-ready `EXAMPLES.md` you can drop into `crates/svc-interop/docs/` (or wherever you keep crate docs). It’s provider-agnostic, focuses on top payment players and important services, and frames everything as **illustrative examples** (not endorsements).

---

# EXAMPLES.md — Third-Party Webhook Adapters (Illustrative)

> These examples show how to map external providers to `svc-interop`’s generic webhook contract:
>
> `POST /webhooks/:provider`
>
> * **Provider slug** becomes `:provider` (e.g., `stripe`, `github`).
> * **Ingress verification** follows the provider’s spec (usually HMAC or public-key signature).
> * After verification, events are normalized, capability-checked, and emitted on the bus.
> * Body size, chunking, timeouts, and idempotency follow project invariants.

This file is **non-exclusive**. Being listed here is not an endorsement, and not being listed is not an exclusion. It’s a jump-start catalog of common integrations.

---

## Verification Norms (Edge)

* Prefer provider’s official verification scheme (HMAC header, detached JWS/JWT, RSA/ECDSA signature, or challenge/response).
* Enforce idempotency using provider delivery IDs where available.
* Normalize internal hashing to BLAKE3-256 after ingress verification.
* Default-deny: each adapter must be explicitly enabled in config.

---

## Quick Reference Legend

* **Slug** → the path segment used at `POST /webhooks/:provider`
* **Signature / Verify** → primary header/field and scheme (illustrative)
* **Notable Events** → common event types you’ll typically see

> Header names and schemes can change; always consult the provider’s docs when implementing an adapter.

---

## A) Payments, Billing & Commerce (Top/Global)

| Provider                       | Slug                | Signature / Verify (illustrative)                                                     | Notable Events                                                                        |
| ------------------------------ | ------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Stripe                         | `stripe`            | `Stripe-Signature` (HMAC-SHA256, signed payload timestamp)                            | `payment_intent.succeeded`, `charge.refunded`, `customer.subscription.updated`        |
| PayPal                         | `paypal`            | `PAYPAL-TRANSMISSION-ID/TIME/SIG`, `PAYPAL-CERT-URL`, `PAYPAL-AUTH-ALGO` (cert-based) | `PAYMENT.SALE.COMPLETED`, `CHECKOUT.ORDER.APPROVED`, `BILLING.SUBSCRIPTION.CANCELLED` |
| Adyen                          | `adyen`             | HMAC-SHA256 (signature typically in payload; base64)                                  | `AUTHORISATION`, `CANCELLATION`, `REFUND`, `CHARGEBACK`                               |
| Braintree                      | `braintree`         | `BT_SIGNATURE` + `BT_PAYLOAD` (HMAC verify)                                           | `transaction_disbursed`, `dispute_opened`, `subscription_charged_successfully`        |
| Square                         | `square`            | `X-Square-Signature` (HMAC-SHA256)                                                    | `payment.created`, `refund.updated`, `invoice.paid`                                   |
| Shopify Payments (via Shopify) | `shopify`           | `X-Shopify-Hmac-Sha256` (HMAC-SHA256), topic in `X-Shopify-Topic`                     | `orders/paid`, `refunds/create`, `checkouts/update`                                   |
| Chargebee                      | `chargebee`         | `Chargebee-Event-Signature` (HMAC-SHA256)                                             | `subscription_renewed`, `invoice_generated`, `payment_refunded`                       |
| Recurly                        | `recurly`           | `X-Recurly-Signature` (HMAC)                                                          | `invoice.created`, `subscription.renewed`, `transaction.refunded`                     |
| Paddle                         | `paddle`            | `p_signature` (RSA public key verify)                                                 | `payment_succeeded`, `subscription_created`, `refund_issued`                          |
| Coinbase Commerce              | `coinbase_commerce` | `X-CC-Webhook-Signature` (HMAC-SHA256)                                                | `charge:confirmed`, `charge:failed`                                                   |
| Klarna                         | `klarna`            | `Klarna-Signature` (HMAC-SHA256)                                                      | `order.captured`, `order.refunded`                                                    |
| Affirm                         | `affirm`            | `X-Affirm-Signature` (HMAC)                                                           | `charge.created`, `charge.refunded`                                                   |
| Razorpay                       | `razorpay`          | `X-Razorpay-Signature` (HMAC-SHA256)                                                  | `payment.captured`, `refund.processed`, `order.paid`                                  |
| Paystack                       | `paystack`          | `X-Paystack-Signature` (HMAC-SHA512)                                                  | `charge.success`, `invoice.create`, `transfer.success`                                |
| Flutterwave                    | `flutterwave`       | `verif-hash` (HMAC)                                                                   | `charge.completed`, `transfer.completed`, `refund.completed`                          |
| Mercado Pago                   | `mercadopago`       | `x-signature` (HMAC; params include `sha256=`)                                        | `payment.updated`, `chargebacks.created`                                              |
| Mollie                         | `mollie`            | `X-Mollie-Signature` (HMAC)                                                           | `payment.paid`, `refund.created`, `subscription.canceled`                             |
| Checkout.com                   | `checkoutcom`       | `Cko-Signature` (HMAC-SHA256)                                                         | `payment_captured`, `payment_refunded`, `chargeback_updated`                          |
| Worldpay                       | `worldpay`          | HMAC header (varies by account profile)                                               | `payment_settled`, `refund_processed`                                                 |
| Authorize.Net                  | `authorize_net`     | `X-ANET-Signature` (HMAC-SHA512)                                                      | `net.authorize.payment.authcapture.created`                                           |
| Wise                           | `wise`              | `X-Signature` (HMAC)                                                                  | `transfer.state_change`, `balance.credit`                                             |

---

## B) Banking, Open Finance & Aggregators

| Provider  | Slug        | Signature / Verify                                      | Notable Events                             |
| --------- | ----------- | ------------------------------------------------------- | ------------------------------------------ |
| Plaid     | `plaid`     | `Plaid-Verification` (detached JWS), `Plaid-Request-Id` | `TRANSACTIONS`, `ITEM`, `AUTH` updates     |
| Tink      | `tink`      | HMAC header (per app secret)                            | `transactions.updated`, `accounts.updated` |
| TrueLayer | `truelayer` | `Tl-Signature` (HMAC/JWS depending on setup)            | `payment.settled`, `refund.executed`       |

---

## C) Stores & Platforms

| Provider    | Slug          | Signature / Verify                     | Notable Events                                        |
| ----------- | ------------- | -------------------------------------- | ----------------------------------------------------- |
| Shopify     | `shopify`     | `X-Shopify-Hmac-Sha256`                | `orders/create`, `orders/fulfilled`, `refunds/create` |
| WooCommerce | `woocommerce` | `X-WC-Webhook-Signature` (HMAC-SHA256) | `order.created`, `order.updated`, `product.updated`   |
| BigCommerce | `bigcommerce` | `X-Bc-Signed` (HMAC)                   | `store/order/statusUpdated`, `store/customer/created` |

---

## D) Dev Platforms (Source, CI/CD)

| Provider          | Slug        | Signature / Verify                                                              | Notable Events                                     |
| ----------------- | ----------- | ------------------------------------------------------------------------------- | -------------------------------------------------- |
| GitHub            | `github`    | `X-Hub-Signature-256` (HMAC-SHA256), plus `X-GitHub-Event`, `X-GitHub-Delivery` | `push`, `pull_request`, `release`, `check_suite`   |
| GitLab            | `gitlab`    | `X-Gitlab-Token` (secret token) or `X-Gitlab-Signature`                         | `Push Hook`, `Merge Request Hook`, `Tag Push Hook` |
| Bitbucket         | `bitbucket` | `X-Hub-Signature` (HMAC-SHA256)                                                 | `repo:push`, `pullrequest:created`                 |
| CircleCI          | `circleci`  | `Circleci-Signature` (HMAC)                                                     | `workflow-completed`                               |
| Jenkins (generic) | `jenkins`   | Secret token in `X-Hub-Signature`/query                                         | Job/build status callbacks                         |

---

## E) Messaging, Email & Chat

| Provider | Slug       | Signature / Verify                                               | Notable Events                                              |
| -------- | ---------- | ---------------------------------------------------------------- | ----------------------------------------------------------- |
| Twilio   | `twilio`   | `X-Twilio-Signature` (HMAC over URL+params)                      | `message.status`, `call.status`, `recording.completed`      |
| SendGrid | `sendgrid` | `X-Twilio-Email-Event-Webhook-Signature` + `Timestamp` (Ed25519) | `delivered`, `opened`, `dropped`, `spamreport`              |
| Mailgun  | `mailgun`  | `X-Mailgun-Signature` + `Timestamp` (HMAC-SHA256)                | `delivered`, `opened`, `failed`                             |
| Postmark | `postmark` | `X-Postmark-Webhook-Signature`                                   | `Delivery`, `Open`, `Bounce`, `SpamComplaint`               |
| Slack    | `slack`    | `X-Slack-Signature` + `X-Slack-Request-Timestamp` (HMAC-SHA256)  | `event_callback` (app_mention, message), `url_verification` |
| Discord  | `discord`  | `X-Signature-Ed25519` + `X-Signature-Timestamp`                  | `MESSAGE_CREATE`, `INTERACTION_CREATE`                      |

---

## F) Identity & Access

| Provider   | Slug         | Signature / Verify                                                    | Notable Events                                   |
| ---------- | ------------ | --------------------------------------------------------------------- | ------------------------------------------------ |
| Auth0      | `auth0`      | Varies (token introspection, log streams via HTTPS; optional signing) | `success.login`, `failed.login`, `api.operation` |
| Okta       | `okta`       | Event Hooks with verification challenge; optional signed headers      | `user.lifecycle`, `system.logs`                  |
| Clerk      | `clerk`      | `Svix`-style signature headers (HMAC)                                 | `user.created`, `session.ended`                  |
| FusionAuth | `fusionauth` | Secret header token (HMAC)                                            | `user.create`, `jwt.refresh-token.issued`        |

---

## G) Observability, Incidents & Quality

| Provider  | Slug        | Signature / Verify                     | Notable Events                            |
| --------- | ----------- | -------------------------------------- | ----------------------------------------- |
| Datadog   | `datadog`   | `DD-Signature` + `DD-Timestamp` (HMAC) | `monitor.triggered`, `ci.job.completed`   |
| New Relic | `newrelic`  | Secret header token                    | Alerts, violations, deployments           |
| Sentry    | `sentry`    | `Sentry-Hook-Signature` (HMAC)         | `event_alert.triggered`, `issue.assigned` |
| PagerDuty | `pagerduty` | `X-PagerDuty-Signature`                | `incident.trigger`, `incident.resolve`    |

---

## H) Documents, Meetings & Files

| Provider            | Slug       | Signature / Verify                                                     | Notable Events                              |
| ------------------- | ---------- | ---------------------------------------------------------------------- | ------------------------------------------- |
| DocuSign            | `docusign` | `X-DocuSign-Signature-1` (HMAC)                                        | `envelope-completed`, `recipient-completed` |
| Zoom                | `zoom`     | `ZOOM-Signature` (HMAC)                                                | `meeting.started`, `recording.completed`    |
| Dropbox             | `dropbox`  | `X-Dropbox-Signature` (HMAC-SHA256)                                    | `file.change`, `folder.change`              |
| Box                 | `box`      | `BOX-Signature` (HMAC)                                                 | `FILE.UPLOADED`, `FILE.PREVIEWED`           |
| Google Drive (Push) | `gdrive`   | `X-Goog-Resource-State/Id` + channel tokens (verification via channel) | `change` notifications                      |

---

## I) Shipping, Tax, Accounting, Support

| Provider          | Slug         | Signature / Verify                   | Notable Events                                     |
| ----------------- | ------------ | ------------------------------------ | -------------------------------------------------- |
| Shippo            | `shippo`     | Secret header token                  | `track_updated`, `transaction_created`             |
| EasyPost          | `easypost`   | `X-Hmac-Signature` (HMAC)            | `scan_form.created`, `tracker.updated`             |
| Avalara           | `avalara`    | Secret token                         | Tax calc result hooks                              |
| TaxJar            | `taxjar`     | Secret token                         | Order tax updates (where supported)                |
| QuickBooks Online | `quickbooks` | `X-Intuit-Signature` (HMAC-SHA256)   | `invoice.create`, `payment.update`                 |
| Xero              | `xero`       | `x-xero-signature` (HMAC)            | `invoices.updated`, `contacts.updated`             |
| Zendesk           | `zendesk`    | `X-Zendesk-Webhook-Signature` (HMAC) | `ticket.created`, `ticket.updated`                 |
| Intercom          | `intercom`   | `X-Hub-Signature` (HMAC)             | `conversation.user.created`, `contact.tag.created` |

---

## Provider Slug Registry (Illustrative)

```text
stripe, paypal, adyen, braintree, square, shopify, chargebee, recurly, paddle,
coinbase_commerce, klarna, affirm, razorpay, paystack, flutterwave, mercadopago,
mollie, checkoutcom, worldpay, authorize_net, wise,
plaid, tink, truelayer,
woocommerce, bigcommerce,
github, gitlab, bitbucket, circleci, jenkins,
twilio, sendgrid, mailgun, postmark, slack, discord,
auth0, okta, clerk, fusionauth,
datadog, newrelic, sentry, pagerduty,
docusign, zoom, dropbox, box, gdrive,
shippo, easypost, avalara, taxjar, quickbooks, xero, zendesk, intercom
```

---

## Minimal Adapter Template (Pseudo-Spec)

When adding a new provider:

1. **Config knobs**

   * `providers.<slug>.enabled = true|false`
   * `providers.<slug>.secret_env = "ENV_VAR_NAME"` (or key ID + PKI material as required)
   * `providers.<slug>.allowed_hosts = ["hostname.tld"]` (optional allowlist)
   * `providers.<slug>.tolerance_seconds = 300` (timestamp drift, if applicable)
   * `providers.<slug>.replay_cache_ttl = "15m"` (idempotency window)

2. **Verify (ingress)**

   * Extract signature/timestamp headers as per provider.
   * Recompute digest over the exact canonical string (provider-specific).
   * Constant-time compare; reject on mismatch/expiry.
   * Parse event; capture delivery ID for dedupe.

3. **Normalize & Emit (internal)**

   * Map to internal event schema (`source`, `type`, `subject`, `time`, `data`).
   * Attach a capability/macaroon describing permitted side effects.
   * Publish to the bus; expose metrics (`*_received_total`, `*_verified_total`, `*_dropped_total`).

---

## Example Config Snippet (Stripe)

```toml
[providers.stripe]
enabled = true
secret_env = "STRIPE_WEBHOOK_SECRET"
allowed_hosts = ["api.stripe.com"]   # optional; usually "*"
tolerance_seconds = 300
replay_cache_ttl = "15m"
```

> Replace `stripe` with any slug above and mirror the same knobs. Some providers (e.g., Paddle, PayPal, SendGrid) use **public-key** verification instead of HMAC—store the key material via your KMS or env-backed secrets and adapt the verifier accordingly.

---

## Notes & Caveats

* **Regional PSPs**: Depending on your launch geography, consider adding PayU, iDEAL gateways, Giropay, Bancontact, Swish, or domestic rails. Treat them as standard HMAC or PKI adapters.
* **Retries & Ordering**: Most providers retry on 5xx; many don’t guarantee ordering. Use delivery IDs and durable dedupe.
* **Security**: Validate content type, enforce max body, and never persist provider HMAC secrets or raw signatures beyond what’s needed for audit.
* **Testing**: Provide canned payloads + signatures as test vectors per adapter; add fuzzers for header edge-cases and timestamp drift.

---

