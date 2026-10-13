-- The intel network schema. Every timestamp is timestamptz in UTC (MySQL
-- `timestamp` columns, stored in UTC, carry over unshifted). EVE ids
-- (characters, corporations, alliances) are bigints; alliances and
-- corporations use them as primary keys.

-- Entity search matches names case and accent insensitively.
create extension if not exists unaccent;

create table alliances (
    id bigint primary key,
    name text not null,
    ticker text not null,
    -- When the row was last fetched from ESI; refreshed after 24 hours.
    last_updated timestamptz,
    created_at timestamptz not null,
    updated_at timestamptz not null
);

create table corporations (
    id bigint primary key,
    name text not null,
    ticker text not null,
    alliance_id bigint references alliances (id) on delete set null,
    member_count integer,
    last_updated timestamptz,
    created_at timestamptz not null,
    updated_at timestamptz not null
);

-- One user per EVE character.
create table users (
    id bigserial primary key,
    character_id bigint not null unique,
    character_name text not null,
    character_owner_hash text not null,
    corporation_id bigint references corporations (id) on delete set null,
    alliance_id bigint references alliances (id) on delete set null,
    -- Never written; kept because the user JSON carries it.
    last_active_at timestamptz,
    created_at timestamptz not null,
    updated_at timestamptz not null
);

-- Sanctum's personal access tokens, kept row for row so every desktop
-- client stays signed in: `token` is the SHA-256 (hex) of the part of the
-- plain token after `<id>|`.
create table personal_access_tokens (
    id bigserial primary key,
    tokenable_type text not null check (tokenable_type = 'App\Models\User'),
    tokenable_id bigint not null references users (id) on delete cascade,
    name text not null,
    token char(64) not null unique,
    -- The JSON text Sanctum stored (`["*"]`).
    abilities text,
    last_used_at timestamptz,
    expires_at timestamptz,
    created_at timestamptz not null,
    updated_at timestamptz not null
);

create index personal_access_tokens_tokenable on personal_access_tokens (tokenable_type, tokenable_id);
create index personal_access_tokens_expires_at on personal_access_tokens (expires_at);

-- The website's sessions: the SHA-256 of the cookie's token.
create table sessions (
    id char(64) primary key,
    user_id bigint not null references users (id) on delete cascade,
    last_activity timestamptz not null,
    created_at timestamptz not null
);

create index sessions_last_activity on sessions (last_activity);

create table intel_networks (
    id bigserial primary key,
    name text not null,
    slug text not null unique,
    owner_id bigint not null references users (id) on delete cascade,
    created_at timestamptz not null,
    updated_at timestamptz not null
);

-- Who may use a network: a character, a corporation or an alliance. The
-- type keeps the PHP class names the API has always returned from
-- `POST /api/networks`.
create table network_access (
    id bigserial primary key,
    intel_network_id bigint not null references intel_networks (id) on delete cascade,
    accessible_type text not null check (
        accessible_type in ('App\Models\User', 'App\Models\Corporation', 'App\Models\Alliance')
    ),
    accessible_id bigint not null,
    permission text not null default 'viewer' check (permission in ('viewer', 'member', 'manager')),
    is_owner boolean not null default false,
    expires_at timestamptz,
    created_at timestamptz not null,
    updated_at timestamptz not null,
    constraint network_access_unique unique (intel_network_id, accessible_type, accessible_id)
);

create index network_access_accessible on network_access (accessible_id);

create table intel_entries (
    id bigserial primary key,
    intel_network_id bigint not null references intel_networks (id) on delete cascade,
    entity_type text not null check (entity_type in ('character', 'corporation', 'alliance')),
    entity_id bigint not null,
    entity_name text not null,
    color text,
    label text,
    notes text,
    added_by bigint not null references users (id) on delete cascade,
    created_at timestamptz not null,
    updated_at timestamptz not null,
    constraint intel_entries_unique unique (intel_network_id, entity_type, entity_id)
);

create index intel_entries_entity on intel_entries (entity_id);

create table network_scans (
    id bigserial primary key,
    intel_network_id bigint not null references intel_networks (id) on delete cascade,
    user_id bigint not null references users (id) on delete cascade,
    scan_type text not null check (scan_type in ('local', 'dscan')),
    raw_text text not null,
    solar_system text,
    created_at timestamptz not null,
    updated_at timestamptz not null
);

-- The history, newest first, and the retention purge.
create index network_scans_history on network_scans (intel_network_id, created_at desc, id desc);
create index network_scans_created_at on network_scans (created_at);

-- Shared pilot lists. Codes are unique and looked up ignoring case, as
-- MySQL's collation compared them.
create table shares (
    id bigserial primary key,
    code text not null,
    pilots jsonb not null,
    pilot_count integer not null,
    views bigint not null default 0,
    created_at timestamptz not null,
    updated_at timestamptz not null
);

create unique index shares_code on shares (lower(code));
