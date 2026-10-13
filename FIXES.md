# Fixes

Every Laravel bug the rewrite fixes instead of copying, with the test that
pins the new behavior, then the deliberate changes and what is left for
the owner to decide. Together with the test normalizations
(`tests/integration/common/golden.rs`) this is the complete list of
differences from the Laravel app on `main`.

## Bugs fixed

### Security: members could change and delete other networks' entries and access

This one is live in production until the cutover.

Laravel: the `{entry}` and `{access}` route parameters were resolved by id
alone, without checking that the row belongs to the `{network}` in the
same URL. Only the network was authorized. So any member of one network
could update or delete any entry of any other network with
`PUT|DELETE /api/networks/<own network>/entries/<their entry>`, and any
manager could change or revoke any non-owner access row of any other
network the same way (`PUT|DELETE /api/networks/<own>/access/<theirs>`).
The entry events were even broadcast on the attacker's own network.

Now: an entry or access row that is not in the network of the URL is not
found (404 with the same message as an id that does not exist), before
anything is authorized or changed, as Laravel's `scopeBindings()` would
have answered. Nothing is changed and nothing is broadcast.

Tests: `intel_entry::updating_an_entry_of_another_network_is_not_found`,
`intel_entry::deleting_an_entry_of_another_network_is_not_found`,
`network_access::updating_an_access_of_another_network_is_not_found`,
`network_access::revoking_an_access_of_another_network_is_not_found`.

### A manager could take the owner's rights through the grant form

Laravel: `POST /api/networks/{network}/access` updates an existing row of
the same character, corporation or alliance. Granting "character" access
to the owner's own character overwrote the owner row's permission and
expiry, so a manager could demote the owner to viewer or let the owner's
access expire. `PUT` refused owner rows ("Cannot modify owner access"),
the grant did not.

Now: a grant that would change the owner row answers 403 `{"message":
"Cannot modify owner access"}`, like `PUT`, and changes nothing.

Test: `network_access::granting_access_to_the_owner_is_forbidden`.

### Updating an entry could not clear its label or color

Laravel: `PUT /api/networks/{network}/entries/{entry}` validated `color`
and `label` as `sometimes|string`. An empty string becomes null before
validation, and null failed `string`, so clearing a label answered 422.
The v0.5 desktop client sends every field on update, an empty `color`
for entries without a color, so editing any note-only entry failed
there. Creating an entry accepted the same values (`nullable`).

Now: `color` and `label` may be null (or empty) on update and clear the
value; other values are checked as before. `entity_name` still has to be
a string when it is sent.

Test: `intel_entry::updating_can_clear_the_label_and_color`.

### A guest without `Accept: application/json` got a 500

Laravel: a protected API route (and `/broadcasting/auth`) answered a
request without a token and without a JSON `Accept` header with 500,
because the auth middleware tried to redirect to a `login` route the site
never had (`Route [login] not defined`, in the production log).

Now: 401 `{"message":"Unauthenticated."}` whatever the `Accept` header.

Test: `api_auth::a_guest_without_a_json_accept_header_gets_401`.

### A failed validation without `Accept: application/json` redirected

Laravel: an API request without a JSON `Accept` header that failed
validation was answered with a 302 back to the Referer (or the home page),
the redirect a web form gets; a client never saw the errors.

Now: 422 with the errors as JSON, whatever the `Accept` header.

Test: `share::a_failed_validation_answers_422_without_a_json_accept_header`.

### Revoking a token by a non-numeric id was a 500

Laravel: `DELETE /api/tokens/{tokenId}` typed the parameter as `int`, so
`/api/tokens/abc` failed with a TypeError (500).

Now: 404, like any id that names no token.

Test: `api_tokens::revoking_a_token_by_a_non_numeric_id_is_not_found`.

### Channel auth for a malformed network id was a 500

Laravel: `/broadcasting/auth` for `private-intel-network.abc` called the
channel callback with `int $networkId` and failed with a TypeError (500).

Now: 403, like a network the user may not see.

Test: `broadcasting_auth::a_malformed_network_id_is_forbidden`.

### An ESI hiccup at login removed the user's corporation and alliance

Laravel: when the affiliation lookup failed at login, the user was saved
with no corporation and no alliance, so the pilot lost every network
shared with their corporation or alliance until the next login.

Now: the known corporation and alliance are kept when ESI does not
answer.

Test: `eve_login::an_esi_failure_keeps_the_known_affiliation`.

### A corporation or alliance ESI could not load failed the login

Laravel: when the affiliation lookup worked but the corporation (or
alliance) itself could not be fetched, the user row pointed at a
corporation that did not exist and the foreign key failed the login with
a 500. The same in the hourly refresh job, where it stopped the job for
everyone after that user. A corporation whose alliance was not stored
failed the same way.

Now: an affiliation whose row could not be stored is left empty for
that user (and a corporation's alliance for that corporation), and the
login or the refresh goes on.

Tests: `eve_login::a_corporation_esi_cannot_load_does_not_fail_the_login`,
`affiliations::a_corporation_esi_cannot_load_does_not_stop_the_refresh`.

### A corporation or alliance never fetched was never refreshed

Laravel: the 24 hour refresh compared `last_updated?->diffInHours(now())
> 24`; a row with no `last_updated` (one that never came from ESI)
compared `null > 24` and was never refreshed.

Now: a row with no `last_updated` is fetched like a stale one.

Test: `affiliations::a_row_never_fetched_is_fetched`.

### An expiry with a time zone offset was stored at the wrong moment

Laravel: `expires_at` was parsed with its offset, then stored as its
local time read as UTC, so `2026-12-01T10:00:00+02:00` expired two hours
late.

Now: the moment is kept (stored as `2026-12-01T08:00:00+00:00`).

Test: `network_access::an_expiry_with_an_offset_keeps_its_moment`.

### An expired or replayed EVE login answered 500

Laravel: a callback whose `state` did not match the session (an old tab,
a second click, an expired session) threw Socialite's
InvalidStateException, a 500 page.

Now: 419 "Page Expired", the page Laravel shows for an expired form.

Test: `eve_login::a_callback_with_a_wrong_state_has_expired`.

### Cancelling the EVE login answered 500

Laravel: when the user declined on EVE's page, the callback came back
with `error=access_denied` and no code; exchanging no code failed with a
500.

Now: the browser goes back to the home page.

Test: `eve_login::a_cancelled_login_goes_home`.

### A desktop login left half done turned the next website login into one

Laravel: `/eve?desktop` put `desktop_auth` into the session, and only a
finished callback removed it. A user who abandoned a desktop login and
later signed in on the website got the token page instead.

Now: every start of the login decides the kind of login for that
attempt alone.

Test: `eve_login::a_website_login_after_an_abandoned_desktop_login_signs_in`.

### A GitHub failure slowed every page

Laravel: the latest release was cached for an hour, but a failure
(`null`) was not cached, so while GitHub failed or rate limited the
server every page view asked again, waiting up to 5 seconds.

Now: a failure is remembered for 5 minutes (the page shows `v0.1.0`
meanwhile, as before).

Test: `github::a_failure_is_remembered_for_five_minutes`.

### `SCAN_RETENTION_DAYS=0` deleted every scan

Laravel: the config documents 0 as "keep scans indefinitely", but the
purge deleted every scan older than `now() - 0 days`, so all of them.

Now: 0 keeps every scan, from the setting and from `--days=0`.

Tests: `purge_old_scans::zero_days_keeps_every_scan`.

## Deliberate changes

### The entity search no longer asks ESI

Laravel asked ESI's public `/latest/search/` for a category without
local matches. CCP removed that endpoint (it answers 404), so the call
always found nothing. Every miss still cost a request that ESI counts
against the error budget, which the new server shares with the other EVE
sites on it. The search now answers the same empty list without the
call.

Test: `entity_search::a_query_without_local_matches_asks_nobody`.

### JSON is compact and unescaped

Laravel escaped `/` and non-ASCII characters in JSON bodies
(`https:\/\/`, `é`) and pretty-printed the JSON of exceptions (404,
403, 405). The rewrite sends the same values compact and unescaped;
every client parses JSON. The golden comparisons compare parsed JSON.

Test: `api_errors::error_answers_are_compact_json`.

### Numbers are always numbers

Laravel answered a freshly created entry or access row with the values
as the request sent them, so `"entity_id": "12345"` (a string) came back
as a string. The answers now carry the stored, typed values. The
`integer` rule takes JSON integers and integer strings; it no longer
takes `true` as 1.

Test: `intel_entry::an_entity_id_sent_as_text_is_answered_as_a_number`.

### An id with trailing letters names nothing

MySQL compared `id = '5abc'` as `id = 5`, so `/api/networks/5abc` found
network 5. Ids in paths now have to be whole numbers; anything else is
not found.

Test: `intel_network::an_id_with_trailing_letters_is_not_found`.

### Dates are ISO 8601

`expires_at` took anything PHP's `strtotime` reads ("tomorrow",
"+1 week"). It now takes ISO 8601 dates and date-times (with or without
an offset, `T` or a space) and answers "must be a valid date" for the
rest. No client sends an expiry.

Test: `network_access::an_expiry_must_be_an_iso_date`.

### Website sign-in: a new session, no bridge

The website's session only fed the `auth.user` prop, which no page shows.
Laravel sessions and remember cookies are not carried over; the next
website login starts a `telescope_session` cookie (30 days, HttpOnly,
SameSite=Lax). The `XSRF-TOKEN` cookie is gone, and
`/sanctum/csrf-cookie` answers 204 without cookies. Desktop tokens are
kept row for row and keep working.

Tests: `web_session::a_website_login_signs_the_pages_in`,
`api_misc::the_csrf_cookie_route_answers_without_cookies`.

### Tabs left open on the Laravel site reload

A request with `X-Inertia` (a tab still running the Laravel pages)
answers 409 with `X-Inertia-Location`, so Inertia loads the page in full
from SvelteKit.

Tests: `api_misc::an_inertia_request_reloads_the_page`,
`frontend/e2e/routes.spec.ts` "reloads tabs left open on the Laravel site".

### Pages are rendered on the server

The Inertia pages were rendered in the browser only (the SSR setting
pointed at another site's SSR server). SvelteKit renders them on the
server, so the HTML carries the real titles and content. A page URL with
a trailing slash redirects (308) to the URL without it; the API still
answers both.

Test: `frontend/e2e/routes.spec.ts` "redirects trailing slashes on pages".

### The realtime server is part of the api

Reverb ran as its own daemon. The api now speaks the Pusher protocol 7
itself, at `/app/<key>` on ws.eve-telescope.com (and on the main host),
with the frames Reverb sent (`tests/fixtures/golden/pusher`). Differences:

- Presence channels are refused (`pusher:error` 4009 on subscribe, 403
  from `/broadcasting/auth`); nothing uses them.
- Client events (`client-*`) are not relayed to other subscribers;
  nothing sends them.
- Reverb's HTTP API (`POST /apps/<id>/events`) is not reproduced; only
  Laravel itself called it.

Tests: `pusher_protocol::a_presence_channel_is_refused`,
`pusher_protocol::client_events_are_not_relayed`,
`broadcasting_auth::a_presence_channel_is_refused`.

### Share links use APP_URL

Laravel built the `url` of a new share from the request's host; it is
now `APP_URL` plus `/s/<code>`, the same in production.

Test: `share::creates_a_share_and_answers_its_link`.

### A local demo login

With `DEMO_ENABLED` (on in docker-compose.yml, off in compose.prod.yml)
`/demo-login` signs in as a seeded demo pilot without EVE SSO: with
`?desktop` it shows a desktop token like the EVE login does, without it
it starts a website session. The home page then links to it
(`demoLoginEnabled`). Off, the route is not found.

Tests: `demo::the_demo_login_is_not_found_when_off`,
`demo::the_demo_login_issues_a_desktop_token`.

## Needs owner decision

### The hourly tasks are off

`routes/console.php` schedules three hourly tasks, but the Forge box had
no cron entry and no queue worker for the site, so none of them ever ran
in production: affiliations were refreshed only at login, expired access
rows stayed (every query ignores them), and no scan was ever purged
(1855 rows since April). The rewrite implements all three
(`src/scheduler.rs`, at minute 0 UTC, plus the one-shot commands
`purge_scans`, `purge_expired_access` and `refresh_affiliations`) and
ships with `SCHEDULER_ENABLED=false` in compose.prod.yml, so production
behaves as before. Turning it on means: the first purge deletes every
scan older than `SCAN_RETENTION_DAYS` (7 by default), and the
affiliation refresh can grant or remove corporation and alliance access
for pilots who changed corporation since their last login.

### Access matches ids without their type

A network access row matches a user by `accessible_id` against the
user's character, corporation and alliance ids, ignoring
`accessible_type`. EVE's id ranges for the three never overlap, so this
is safe; kept as it is.

### The websocket host answers the whole API

ws.eve-telescope.com proxied only to Reverb. It now reaches the api, so
`https://ws.eve-telescope.com/api/...` works like the main host. Caddy
could limit that host to `/app/*` and `/up` if wanted.

### `last_active_at` is never written

The user JSON carries `last_active_at`, but nothing ever set it; it stays
null as before.
