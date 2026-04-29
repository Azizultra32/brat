# Git WAL and Snapshots

## WAL ref

- Ref: `refs/grite/wal`
- Each append creates a new commit, parented to the current WAL head.
- Trees contain only WAL data; no working tree files are touched.

## WAL commit tree

```
meta.json
events/YYYY/MM/DD/<chunk>.bin
```

`meta.json` includes:

- `schema_version`
- `actor_id`
- `chunk_hash` (BLAKE2b-256 of the chunk file)
- `prev_wal` (parent commit hash)
- `meta_sig_alg`, `meta_key_id`, `meta_sig` (optional future WAL metadata
  signature)

### Chunk encoding

Chunk files contain a small header and a portable CBOR payload:

- magic: `GRITCHNK`
- version: `u16`
- codec: `cbor-v1`
- payload: canonical CBOR array of `Event` records

`Event` record encoding (fixed-order array):

```
[event_id, issue_id, actor, ts_unix_ms, parent, kind_tag, kind_payload, sig]
```

- `event_id`: 32-byte bstr (BLAKE2b-256 of canonical preimage)
- `issue_id`: 16-byte bstr
- `actor`: 16-byte bstr
- `ts_unix_ms`: u64
- `parent`: null or 32-byte bstr
- `kind_tag`/`kind_payload`: same tags and payloads as in `docs/data-model.md`
- `sig`: null or bstr canonical CBOR signature envelope (optional)

Chunk integrity is verified by `chunk_hash`. Event authorship is verified by
`sig` when present. Future WAL metadata signatures should sign `schema_version`,
`actor_id`, `prev_wal`, `chunk_hash`, and chunk paths as described in
`docs/security-signing.md`.

## Append algorithm

1. Read current `refs/grite/wal` head (if present).
2. Create a new commit with parent = head, adding a new chunk file.
3. Update `refs/grite/wal` to the new commit.
4. Push the ref (optional).

If the push is rejected because the remote advanced:

1. Fetch `refs/grite/wal`.
2. Create a new commit whose parent is the fetched head, containing the same chunk.
3. Push again (fast-forward only).

History is never rewritten.

## Sync

- Pull: `git fetch <remote> refs/grite/*:refs/grite/*`
- Push: `git push <remote> refs/grite/*:refs/grite/*`

## Snapshots (periodic, no daemon required)

Snapshots are optional, monotonic optimization refs that speed rebuilds without changing the WAL.

- Ref format: `refs/grite/snapshots/<unix_ms>`
- A snapshot commit stores a compacted set of events plus a `snapshot.json` metadata file.
- Rebuild uses the latest snapshot, then replays WAL commits after its `wal_head`.

### When snapshots are created

Snapshots are created opportunistically, even without an always-on daemon:

- During `grite sync --push` if WAL growth exceeds a threshold
- During explicit `grite snapshot create` command
- During `grite doctor --fix` if snapshot staleness is detected

When a daemon is running, it may also create snapshots on the same thresholds.

Suggested thresholds (configurable):

- WAL events since last snapshot > 10,000
- OR last snapshot older than 7 days

These snapshot thresholds are separate from local projection rebuild thresholds.
Use `grite --no-daemon db stats --json` for projection signals such as
`events_since_rebuild`, `days_since_rebuild`, `size_bytes`, and
`rebuild_recommended`.

### Snapshot metadata

`snapshot.json` includes:

- `schema_version`
- `created_ts`
- `wal_head` (commit hash)
- `event_count`
- `chunk_hash`
- `snapshot_sig_alg`, `snapshot_key_id`, `snapshot_sig` (optional future
  snapshot metadata signature)

Snapshots are never rewritten; older snapshots can be pruned with
`grite snapshot gc`.
If snapshot signature verification fails, clients should ignore that snapshot
and replay WAL events instead of trying to repair history.
