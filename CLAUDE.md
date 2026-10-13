# eve-telescope.com

The website and server of the Telescope desktop app (EVE Online intel
tool): the landing page, shared scan links, the EVE SSO login that hands
the desktop app its token, and the JSON API plus Pusher protocol
websocket behind its intel networks. A Rust API (Axum + Postgres via
sqlx) and a SvelteKit frontend (SSR, adapter-node, Svelte 5, Tailwind 4)
in `frontend/`. One shared origin (the Vite dev proxy locally, Caddy in
docker and production) routes backend paths and every mutation to Axum
and page GETs to SvelteKit. Never point the browser at Axum's port
directly. The setup mirrors Medico (`../Medico`), body-and-face
(`../body-and-face`) and MutaMarket (`../MutaMarket`, EVE SSO and ESI):
when in doubt about structure or tooling, do what they do.

## The contract with installed clients

Desktop clients cannot be updated with the site, so their wire format is
fixed: every API path, status code and JSON shape (key order included),
Sanctum's token format (`<id>|<secret>`, stored as the SHA-256 of the
secret), the websocket at `/app/<key>` with the public key
`04pcwy13bvcyjoio6mf6`, Reverb's frames byte for byte, event names
`App\Events\IntelEntryCreated` and friends with JSON-string `data`, and
CORS on `/api/*` and `/broadcasting/auth` for the Tauri clients. The
v0.5 client updates entries with PATCH and falls back to PUT on 405, so
PATCH must stay 405. Change none of it without a client release.

## Details that look arbitrary but are not

- A network's entries are listed by entity type then entity id, its
  access rows by the PHP class name (`collate "C"`) then id: the order
  MySQL's unique indexes returned. Scans are newest first, ties by id
  descending. Everything else is by id.
- Freshly created entries and networks answer their keys in the order
  Laravel set them (`entity_type` first, `id` last); stored rows answer in
  column order. `serde_json` keeps insertion order (`preserve_order`);
  use `shift_remove`, never `remove`, on a `Map`.
- Three timestamp forms (`src/dates.rs`): models `.000000Z`, access and
  scan answers `+00:00`.
- The websocket pings only subscribed connections, after a full ping
  interval of silence, and drops them one tick later with error 4201
  (`src/pusher/socket.rs`), as Reverb did.
- adapter-node 6 ignores `ORIGIN` at runtime; the frontend takes the
  scheme from `PROTOCOL_HEADER` (x-forwarded-proto), so Caddy must pass
  the real one.

## Testing rules

- Every change lands with its tests in the same commit.
- Tests are precise: exact JSON with keys in order at every level
  (`common::assert_json_eq`), exact messages, statuses and orderings,
  with the expected values written out in the test.
- Behavior tests drive the real router (`oneshot`) on the dedicated
  `telescope_test` database (`db::test_pool()`, created and migrated on
  first use, `TEST_DATABASE_URL` to override) with a fixed clock
  (`clock::Clock`). ESI, EVE SSO and GitHub are local mocks on ephemeral
  ports (`tests/integration/common/mocks.rs`), never stubs of our own
  code. The websocket suites serve the router on a port and connect with
  tokio-tungstenite (`common/socket.rs`).
- Integration suites are modules of the single `tests/integration`
  binary and run single-threaded (`.cargo/config.toml`): they truncate
  the shared tables. Run one suite with `cargo test <module>::`.
- Frontend logic has vitest suites next to the code (`*.test.ts`, and
  `*.svelte.test.ts` in a browser). Playwright (`frontend/e2e`) runs
  against the composed stack and seeds through `docker exec
  telescope-postgres psql`.
- Before committing: `cargo fmt`, `cargo clippy --all-targets` (zero
  warnings), `cargo test`, and in `frontend/` `npm run check`,
  `npm run lint`, `npm run format:check`, `npm test`.

## Layout and commands

- `src/auth/` Sanctum tokens, the website session, EVE SSO and the login;
  `src/intel/` networks, access, entries, scans, lookup and search;
  `src/shares.rs` share links; `src/pusher/` the websocket server, the
  channel signatures and the events; `src/esi.rs` and
  `src/affiliations.rs` ESI and the corporation/alliance refresh;
  `src/github.rs` the latest release; `src/scheduler.rs` the hourly
  tasks; `src/input.rs` and `src/validation.rs` the request input and the
  rules with Laravel's messages; `src/server/` the router, CORS, error
  answers and the page endpoints (`/api/pages/*`).
- `migrations/` the schema (never edit an applied migration, sqlx checks
  their checksums); `deploy/` the Caddyfiles.
- `frontend/` the SvelteKit app; `frontend/proxy-paths.ts` is the routing
  contract, mirrored by `deploy/Caddyfile.dev` and `deploy/Caddyfile`.
- Development: `docker compose up -d postgres`, `cargo run` (API on
  `BIND_ADDR`, default 127.0.0.1:3000) and `npm run dev` in `frontend/`.
  `.env` for config (see `.env.example`).
- Whole stack: `docker compose up --build` on http://localhost:5318, with
  the demo pilot seeded. Ports are fixed: Postgres 5448, stack 5318.
- `cargo run --bin seed_demo` seeds the demo pilot; `purge_scans
  [--days=N]`, `purge_expired_access` and `refresh_affiliations` run the
  hourly tasks once; `migrate` applies the migrations and exits.
- Production: `compose.prod.yml` runs the api and frontend images on the
  shared server, behind its Caddy and Postgres (see the README).

## Legacy reference

Until the rewrite is merged, the Laravel app on `main` is the reference:
`scripts/laravel-reference/run.sh` runs it in docker (HTTP on 8318,
Reverb on 8319), `record.py` and `pusher.mjs` record the golden fixtures
in `tests/fixtures/golden`, `scripts/parity` prints the parity report.
FIXES.md lists every intended difference and tests/PORTED.md maps the
PHP tests.
