# Security Signing

This document defines the signing design for Grite events. It is a design
contract for implementation work; current unsigned events remain valid unless a
caller opts into strict verification.

## Goals

- Bind each event to the actor key that authored it.
- Preserve current `event_id` semantics: `sig` is never part of the event hash.
- Verify WAL chunks and snapshots without treating projection rebuilds as a
  security repair.
- Allow migration from unsigned history without rewriting `refs/grite/*`.

## What Is Signed

### Event Signature

Each signed event stores a canonical signature envelope in `Event.sig`.

`event_id` is still BLAKE2b-256 of the unsigned canonical event preimage from
`docs/data-model.md`. The signature payload signs the event identity and actor
binding, not the serialized event including `sig`.

Event signature payload v1:

```
[
  "grite-event-signature-v1",
  event_id,        // 32-byte bstr
  actor,           // 16-byte bstr
  issue_id,        // 16-byte bstr
  ts_unix_ms,      // u64
  parent,          // null or 32-byte bstr
  signed_at_ms,    // u64
  key_id,          // BLAKE2b-256(public_key), 32-byte bstr
  sig_alg          // "ed25519-v1"
]
```

The envelope stored in `sig` is canonical CBOR:

```
[
  1,               // signature envelope version
  "ed25519-v1",
  key_id,
  public_key,      // optional in trusted-key mode, present in TOFU mode
  signature,       // Ed25519 signature over the payload above
  signed_at_ms
]
```

Initial algorithm: Ed25519. Future algorithms must use a new `sig_alg` string
and must not change the event hash preimage.

Verification must recompute `event_id` from the unsigned canonical event body
before signature validation. If the recomputed hash does not match the
recorded `event_id`, the event is invalid even if its signature verifies over
the recorded `event_id`.

### WAL Chunk Metadata

Event signatures prove event authorship. WAL metadata proves append/package
integrity. WAL `meta.json` should eventually include:

- `meta_sig_alg`
- `meta_key_id`
- `meta_sig`

The WAL metadata signature signs:

```
[
  "grite-wal-meta-signature-v1",
  schema_version,
  actor_id,
  prev_wal,
  chunk_hash,
  chunk_paths
]
```

This catches chunk substitution or mismatched metadata. It is not a replacement
for verifying each event signature.

### Snapshot Metadata

Snapshots are rebuild accelerators. A snapshot may include:

- `snapshot_sig_alg`
- `snapshot_key_id`
- `snapshot_sig`

The snapshot signature signs:

```
[
  "grite-snapshot-signature-v1",
  schema_version,
  created_ts,
  wal_head,
  event_count,
  chunk_hash
]
```

If snapshot verification fails, clients must ignore the snapshot and replay WAL
events. They must not rewrite WAL history or treat `grite rebuild` as a fix for
invalid signatures.

## Keys And Trust

Each actor owns one or more signing keys. Private keys live only in the actor's
local directory:

```
.git/grite/actors/<actor_id>/keys/
```

Public key material may appear in the signature envelope for trust-on-first-use
mode, and should also be exportable for explicit trust setup.

Trust modes:

- `off`: do not verify signatures. Suitable only for compatibility.
- `warn` default during migration: verify when signatures are present, warn on
  unsigned, unknown-key, expired-key, or invalid-signature states.
- `strict`: reject invalid signatures and reject unsigned events unless the
  trust policy explicitly allows legacy ranges.

Trust policy is local operator configuration, not remote-controlled WAL state.
Suggested local file:

```
.git/grite/trust.toml
```

Trust-on-first-use pins the first `(actor, key_id, public_key)` tuple observed
locally and warns if the actor later emits a different untrusted key. Explicit
trust mode accepts only configured actor keys.

## Rotation And Revocation

Rotation should be monotonic:

1. Add a new public key for the actor.
2. Sign new events with the new key.
3. Keep the old key trusted through an overlap window.
4. Mark the old key retired locally after the overlap.

If the old key is available, the rotation statement should be signed by both old
and new keys. If the old key is compromised or unavailable, operators must use
explicit local trust policy to revoke it. Revocation must not rewrite existing
events; it changes verification status from a policy point forward.

Open implementation choice: actor-key lifecycle can be represented as new event
kinds (`ActorKeyAdded`, `ActorKeyRetired`) or as separate actor metadata refs.
Either way, local trust policy remains authoritative for what is accepted.

## Existing Unsigned Events

Existing events with `sig = null` are legacy unsigned events.

Migration behavior:

- Default `warn` mode accepts unsigned events but reports them as `unsigned`.
- `strict` mode may allow an explicit legacy cutoff such as
  `allow_unsigned_through_wal = <commit>` or
  `allow_unsigned_event_ids = [<event_id>, ...]`. Do not use event timestamps as
  strict-mode cutoffs because unsigned events control `ts_unix_ms`.
- Historical WAL events cannot be signed in place because `sig` is stored inside
  the WAL event record and would change chunk bytes and `chunk_hash`.
- Optional backfill must use sidecar signature metadata keyed by `event_id`, and
  must not change `event_id`, event ordering, WAL parents, chunk hashes, or
  existing refs.

## Verification Status

Verification produces one status per event:

- `verified`: signature is valid and the key is trusted.
- `unsigned`: no signature envelope is present.
- `unknown_key`: signature is cryptographically valid but the key is not trusted.
- `expired_key`: signature is valid but the key is outside local trust policy.
- `invalid_signature`: signature does not verify for the event payload.
- `malformed_signature`: envelope is not valid canonical CBOR or references an
  unsupported algorithm.

`invalid_signature` and `malformed_signature` are integrity failures. Projection
rebuilds and snapshot GC do not repair them.

## CLI And Brat Hooks

Future Grite commands:

- `grite key init --alg ed25519 --json`
- `grite key list --json`
- `grite key export <key_id> --json`
- `grite trust add <actor_id> <key_id> --public-key <hex> --json`
- `grite trust revoke <actor_id> <key_id> --json`
- `grite verify [--strict] [--since <ref|event_id>] --json`
- `grite doctor --json` should include verification summary checks.

Future Brat hooks:

- `brat doctor --check --json` should surface a
  `gritee_signature_verification` check once `grite verify` exists.
- `brat status --json` should include an intervention when verification is
  `invalid_signature` or strict mode rejects unsigned events.
- Worker spawn should inherit the actor/key selected for the task worktree.

Expected verification JSON shape:

```
{
  "verified": 1200,
  "unsigned": 80,
  "unknown_key": 2,
  "invalid_signature": 0,
  "malformed_signature": 0,
  "policy": "warn"
}
```

## Risks And Open Questions

- Trust bootstrap is social, not cryptographic. TOFU detects later key changes
  but cannot prove the first key was correct.
- Actor IDs are random identifiers, not human identity proof.
- Private-key storage needs platform-specific protection before strict mode is
  safe for high-trust deployments.
- Multi-device actors need either shared keys or explicit multi-key actor trust.
- Snapshot signatures are optimization checks; WAL event verification remains
  the source of truth.
- Remote servers should not be trusted to decide local trust policy.
