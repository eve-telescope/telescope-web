# Telescope Web

[![CI](https://github.com/eve-telescope/telescope-web/actions/workflows/ci.yml/badge.svg)](https://github.com/eve-telescope/telescope-web/actions/workflows/ci.yml)
[![Release](https://github.com/eve-telescope/telescope-web/actions/workflows/release.yml/badge.svg)](https://github.com/eve-telescope/telescope-web/actions/workflows/release.yml)
[![Website](https://img.shields.io/website?url=https%3A%2F%2Feve-telescope.com&label=eve-telescope.com)](https://eve-telescope.com)
[![Desktop app](https://img.shields.io/github/v/release/eve-telescope/telescope-app?label=desktop%20app)](https://github.com/eve-telescope/telescope-app/releases/latest)
[![Rust](https://img.shields.io/badge/Rust-Axum-DEA584?logo=rust&logoColor=white)](https://github.com/tokio-rs/axum)
[![SvelteKit](https://img.shields.io/badge/SvelteKit-Svelte%205-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev/docs/kit)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-18-4169E1?logo=postgresql&logoColor=white)](https://www.postgresql.org)
[![License](https://img.shields.io/badge/license-MIT-blue)](#license)

The website and server behind [Telescope](https://github.com/eve-telescope/telescope-app),
the desktop intel tool for EVE Online. It serves the landing page and
shared scan links, logs pilots in with EVE SSO and hands the desktop app
its token, and runs the API and the realtime channel of the app's intel
networks.

## Features

- Landing page with the latest desktop release from GitHub
- Share links (`/s/<code>`): a pilot list that opens in the desktop app
  (`telescope://s/<code>`), with a copy fallback and a view counter
- EVE SSO login: a token for the desktop app (`/eve?desktop`, deep-linked
  as `telescope://auth?token=...`) or a website session
- Intel networks for the desktop app: sharing with characters,
  corporations or alliances, viewer, member and manager permissions with
  optional expiry, tagged entries, shared local and d-scans, bulk lookup
  and entity search
- Realtime updates over the Pusher protocol 7 at
  `wss://ws.eve-telescope.com/app/<key>`, private channels signed by
  `/broadcasting/auth`
- Hourly ESI refresh of corporations and alliances, expired access and old
  scans (with `SCHEDULER_ENABLED`)

Installed desktop clients cannot be updated with the site, so the API
paths, JSON shapes, token format and websocket frames are a fixed
contract, pinned by the test suites.

## Stack

- API: Rust, [Axum](https://github.com/tokio-rs/axum) and
  [sqlx](https://github.com/launchbadge/sqlx) on Postgres, with the
  websocket server built in
- Frontend: [SvelteKit](https://svelte.dev/docs/kit) (SSR, adapter-node),
  Svelte 5 and Tailwind CSS 4
- External: EVE SSO, ESI and GitHub's releases API

Caddy (or the Vite dev proxy) puts both behind one origin: the paths in
`frontend/proxy-paths.ts` and every non-GET request go to Axum, page loads
to SvelteKit. `deploy/Caddyfile` holds the same list.

## Getting started

The whole stack (Postgres, API, SvelteKit, Caddy) on
http://localhost:5318, with a demo pilot seeded:

```bash
docker compose up --build -d
```

`/demo-login` signs the website in as the demo pilot, `/demo-login?desktop`
shows a desktop token for it. `docker compose run --rm demo-seed` resets
the demo rows.

Or natively, with only Postgres in docker:

```bash
docker compose up -d postgres     # Postgres on 127.0.0.1:5448
cargo run                         # API on 127.0.0.1:3000
cd frontend && npm install && npm run dev
```

Configuration lives in `.env` (see `.env.example`). The EVE login needs
`EVE_CLIENT_ID` and `EVE_CLIENT_SECRET` of an application whose callback
is `EVE_CALLBACK`.

## Testing

```bash
docker compose up -d postgres
cargo test
cd frontend && npm run check && npm run lint && npm run format:check && npm test
docker compose up --build -d && cd frontend && npm run test:e2e
```

The Rust suites drive the real router on their own `telescope_test`
database with a fixed clock, against mock ESI, EVE SSO and GitHub servers,
and talk to the websocket server with a real client. Playwright runs
against the composed stack, the realtime suite with pusher-js and
laravel-echo.

## Deployment

Production runs on Coolify from `compose.prod.yml`:

1. A merge to main runs `.github/workflows/release.yml`, which builds the
   api, frontend and proxy images and pushes them to
   `ghcr.io/eve-telescope/telescope-*` (`latest` and the commit).
2. The workflow then calls Coolify's deploy webhook (`COOLIFY_WEBHOOK`,
   `COOLIFY_TOKEN`), and Coolify pulls the images and restarts the
   services.
3. The api runs its migrations on startup.

Only the proxy has domains: https://eve-telescope.com and
https://ws.eve-telescope.com for the desktop clients' websocket. Traefik
terminates TLS in front of it and `deploy/Caddyfile` routes by host. The
api reaches the shared Coolify Postgres over the `coolify` network.
`DATABASE_URL`, `EVE_CLIENT_ID`, `EVE_CLIENT_SECRET` and
`PUSHER_APP_SECRET` are Coolify environment variables.

## Layout

```
src/                 the API: auth, intel networks, shares, search, realtime, router
src/pusher/          the Pusher protocol server and channel auth
src/bin/             migrate, seed_demo and the hourly tasks as commands
migrations/          the Postgres schema
tests/integration/   the Rust suites, one binary
frontend/            the SvelteKit app and its Playwright suite (e2e/)
deploy/              the Caddyfile and proxy image
```

## License

MIT

Not affiliated with CCP Games. EVE Online and all related logos are
trademarks of CCP hf.
