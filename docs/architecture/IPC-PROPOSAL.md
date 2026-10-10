# Headless engine IPC proposal (G0 only)

**Status: PROPOSED, NOT FROZEN** — see A1-02 and ADR-004. The native engine is a separate process with stdin/stdout UTF-8 lines. The renderer must interact through a restricted, validated Electron main-process bridge rather than launching subprocesses itself.

## Health exchange

Start the binary with `map2model-engine --stdio`, then send one JSON object per line:

```json
{"apiVersion":"1.0.0","requestId":"health-1","operation":"health"}
```

The headless stub responds:

```json
{"apiVersion":"1.0.0","requestId":"health-1","status":"ok","capabilities":[]}
```

All model-generation operations currently return a typed `UNSUPPORTED_OPERATION` error. They are **not implemented**. Bad JSON, missing/invalid request IDs and overly large parsed lines respond with `INVALID_REQUEST`. Error responses contain a stable `reasonCode` and message; they do not echo untrusted request bodies.

## Security notes

- This is a **demonstration of JSON line framing**, not the final G1 protocol. The implementation detects lines longer than 1 MiB after read, not before allocating input memory; production must bound input bytes during transport parsing.
- Request IDs are limited to 128 ASCII letters/numbers/period/underscore/colon/hyphen. Correlation IDs, cancellation, binary elevation transfer, launch secrets and on-disk permission tokens are still open design items.
- IPC messages must carry a bounded, vetted operation name; UI code must not pass arbitrary filesystem paths, URLs or process commands.
- The `--health` command and passing unit tests do **not** establish terrain-decoding or 3MF-export capability.
