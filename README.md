# Telescope Web

[![CI](https://github.com/eve-telescope/telescope-web/actions/workflows/ci.yml/badge.svg)](https://github.com/eve-telescope/telescope-web/actions/workflows/ci.yml)
[![Website](https://img.shields.io/website?url=https%3A%2F%2Feve-telescope.com&label=eve-telescope.com)](https://eve-telescope.com)
[![License](https://img.shields.io/badge/license-MIT-blue)](#license)
[![Rust](https://img.shields.io/badge/Rust-Axum-DEA584?logo=rust&logoColor=white)](https://github.com/tokio-rs/axum)
[![SvelteKit](https://img.shields.io/badge/SvelteKit-Svelte%205-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev/docs/kit)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-18-4169E1?logo=postgresql&logoColor=white)](https://www.postgresql.org)

The website and server of [Telescope](https://github.com/eve-telescope/telescope-app),
the intel tool for EVE Online: the landing page, shared scan links, the
EVE login for the desktop app, and the API and realtime channel behind
its intel networks.

## Features

- Landing page with the latest desktop release from GitHub
- Share links (`/s/<code>`): a pilot list that opens in the desktop app
  (`telescope://s/<code>`), with a copy fallback and a view counter
- Login with EVE SSO: a token for the desktop app (`/eve?desktop`, shown
  and deep-linked as `telescope://auth?token=...`) or a website session
- Intel networks for the desktop app: networks shared with characters,
  corporations or alliances, viewer, member and manager permissions with
  optional expiry, tagged entries (color, label, notes), shared local and
  d-scans with paged history, bulk lookup and entity search
- Realtime updates over the Pusher protocol 7 (the protocol Laravel
  Reverb spoke) at `wss://ws.eve-telescope.com/app/<key>`, with private
  channels signed by `/broadcasting/auth`
- Hourly tasks: corporation and alliance refresh from ESI, expired access
  and old scans (off unless `SCHEDULER_ENABLED`)
- A one-click demo login with sample networks, for local use

## Stack

- API: Rust, [Axum](https://github.com/tokio-rs/axum) and
  [sqlx](https://github.com/launchbadge/sqlx) on Postgres; the websocket
  server is part of it
- Frontend: [SvelteKit](https://svelte.dev/docs/kit) (SSR, adapter-node),
  Svelte 5 and Tailwind CSS 4
- External: EVE SSO, ESI and GitHub's releases API

One origin serves both: Caddy (or the Vite dev proxy) sends `/api/*`,
`/up`, `/eve`, `/broadcasting/auth`, `/app/*` and the rest of the list in
`frontend/proxy-paths.ts`, and every request that is not a GET or HEAD,
to Axum, and page loads to SvelteKit. The two Caddyfiles in `deploy/`
hold the same list.

## Getting started

The whole stack (Postgres, API, SvelteKit, Caddy) on
http://localhost:5318:

```bash
docker compose up --build -d
```

The `demo-seed` service seeds a demo pilot with two networks, entries,
scans and a share. `/demo-login?desktop` shows a desktop token for it
(paste it into a desktop app built against this server), `/demo-login`
signs the website in. Running the seed again resets the demo rows:

```bash
docker compose run --rm demo-seed
```

Or natively, with only Postgres in docker:

```bash
docker compose up -d postgres     # Postgres on 127.0.0.1:5448
cargo run                         # API on 127.0.0.1:3000
cd frontend && npm install && npm run dev
```

Configuration lives in `.env` (see `.env.example`); everything has a
working local default. The EVE login needs `EVE_CLIENT_ID` and
`EVE_CLIENT_SECRET` of an application whose callback is `EVE_CALLBACK`.

## Testing

```bash
docker compose up -d postgres
cargo test
cd frontend && npm run check && npm run lint && npm run format:check && npm test
docker compose up --build -d && cd frontend && npm run test:e2e
```

The Rust suites drive the real router on their own `telescope_test`
database with a fixed clock, against mock ESI, EVE SSO and GitHub servers
on ephemeral ports, and talk to the websocket server with a real
websocket client. Playwright runs against the composed stack on port
5318, the realtime suite with pusher-js and laravel-echo.

## Deployment

Production runs on a shared server whose Caddy and Postgres serve several
sites. `compose.prod.yml` starts only the API and the frontend, from
images tagged with the commit, on the server's external `web` network:

```bash
docker build -t telescope-api:$TAG .
docker build -t telescope-frontend:$TAG frontend
# in /opt/telescope, with IMAGE_TAG=$TAG and the production values in .env
docker compose -f compose.prod.yml up -d
```

`DATABASE_URL`, `EVE_CLIENT_ID`, `EVE_CLIENT_SECRET` and
`PUSHER_APP_SECRET` go into `.env` next to the compose file; the Pusher
key defaults to the one every desktop client ships with. The API runs its
migrations on startup (`migrate` applies them on its own and exits).
`deploy/Caddyfile` holds the site blocks for eve-telescope.com, its
`www.` redirect and ws.eve-telescope.com; the server's Caddy keeps a copy
of it, so change both.

## Layout

```
src/                 the API: auth, intel networks, shares, search, realtime, router
src/pusher/          the Pusher protocol server and channel auth
src/bin/             migrate, seed_demo and the hourly tasks as commands
migrations/          the Postgres schema
tests/integration/   the Rust suites, one binary
tests/fixtures/      routes, SSO test key and recorded answers
frontend/            the SvelteKit app and its Playwright suite (e2e/)
deploy/              Caddyfiles for the local stack and production
```

## License

MIT

Not affiliated with CCP Games. EVE Online and all related logos are
trademarks of CCP hf.
