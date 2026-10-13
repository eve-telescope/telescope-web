// Shared helpers for the e2e suite: everything talks to the docker stack,
// so the database is seeded and inspected through psql in the
// telescope-postgres container, and desktop tokens are made the way
// Sanctum made them.
import { createHash, randomBytes } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { crc32 } from 'node:zlib';
import { expect, type Page } from '@playwright/test';

/** The Pusher key every desktop client ships with. */
export const PUSHER_KEY = '04pcwy13bvcyjoio6mf6';

export function psql(sql: string): string {
  return execFileSync(
    'docker',
    ['exec', 'telescope-postgres', 'psql', '-U', 'telescope', '-d', 'telescope', '-qtAc', sql],
    { encoding: 'utf8' },
  ).trim();
}

/** A SQL string literal. */
export function quote(value: string | null): string {
  return value === null ? 'null' : `'${value.replaceAll("'", "''")}'`;
}

export function baseUrl(): string {
  return process.env.E2E_BASE_URL ?? 'http://localhost:5318';
}

/** Laravel's RefreshDatabase: every table empty, ids from 1. */
export function resetDatabase(): void {
  psql(
    `truncate alliances, corporations, users, personal_access_tokens, sessions, intel_networks,
       network_access, intel_entries, network_scans, shares restart identity cascade`,
  );
}

let sequence = 0;

export type User = { id: number; characterId: number; name: string };

/** `User::factory()->create()`. */
export function createUser(name?: string): User {
  sequence += 1;
  const characterId = 95_000_000 + sequence;
  const characterName = name ?? `E2E Pilot ${sequence}`;
  const id = Number(
    psql(`insert into users (character_id, character_name, character_owner_hash, created_at, updated_at)
          values (${characterId}, ${quote(characterName)}, 'hash', now(), now()) returning id`),
  );
  return { id, characterId, name: characterName };
}

/** `$user->createToken('Telescope Desktop')->plainTextToken`: the
 * `<id>|<40 random><crc32b>` a desktop client holds. */
export function createToken(user: User): string {
  const random = randomBytes(30)
    .toString('base64')
    .replace(/[^A-Za-z0-9]/g, '')
    .slice(0, 40);
  const secret = `${random}${crc32(random).toString(16).padStart(8, '0')}`;
  const hash = createHash('sha256').update(secret).digest('hex');
  const id = psql(`insert into personal_access_tokens (tokenable_type, tokenable_id, name, token,
      abilities, created_at, updated_at)
    values ('App\\Models\\User', ${user.id}, 'Telescope Desktop', '${hash}', '["*"]', now(), now())
    returning id`);
  return `${id}|${secret}`;
}

/** A network the user owns, as `POST /api/networks` makes it. */
export function createNetwork(owner: User, name = 'E2E Network'): number {
  const id = Number(
    psql(`insert into intel_networks (name, slug, owner_id, created_at, updated_at)
          values (${quote(name)}, ${quote(`e2e-${sequence}-${Date.now()}`)}, ${owner.id}, now(), now())
          returning id`),
  );
  grant(id, owner, 'manager', true);
  return id;
}

export function grant(network: number, user: User, permission: string, owner = false): void {
  psql(`insert into network_access (intel_network_id, accessible_type, accessible_id, permission,
          is_owner, created_at, updated_at)
        values (${network}, 'App\\Models\\User', ${user.characterId}, '${permission}', ${owner}, now(), now())`);
}

/** `Share::factory()->create()`. */
export function createShare(code: string, pilots: string[], views = 0): void {
  psql(`insert into shares (code, pilots, pilot_count, views, created_at, updated_at)
        values (${quote(code)}, ${quote(JSON.stringify(pilots))}::jsonb, ${pilots.length}, ${views},
          '2026-10-04 15:30:00+00', '2026-10-04 15:30:00+00')`);
}

export function shareViews(code: string): number {
  return Number(psql(`select views from shares where code = ${quote(code)}`));
}

/** The demo pilot and its sample networks, seeded by the api image's
 * `seed_demo` (the stack runs with DEMO_ENABLED). */
export function seedDemo(): void {
  execFileSync('docker', ['exec', 'telescope-api', 'seed_demo'], { encoding: 'utf8' });
}

type DatasetRow = Record<string, unknown>;

/** tests/fixtures/dataset.json, the rows the Laravel fixtures were
 * recorded with (the visual comparison). */
export function seedDataset(): void {
  const path = new URL('../../tests/fixtures/dataset.json', import.meta.url);
  const dataset = JSON.parse(readFileSync(path, 'utf8')) as Record<string, DatasetRow[]>;
  resetDatabase();
  for (const table of [
    'alliances',
    'corporations',
    'users',
    'personal_access_tokens',
    'intel_networks',
    'network_access',
    'intel_entries',
    'network_scans',
    'shares',
  ]) {
    for (const row of dataset[table]) {
      const resolved = Object.fromEntries(
        Object.entries(row)
          .filter(([key]) => !key.startsWith('_'))
          .map(([key, value]) => [
            key,
            typeof value === 'string' && /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(value)
              ? `${value}+00`
              : value,
          ]),
      );
      psql(
        `insert into ${table} select * from jsonb_populate_record(null::${table}, ${quote(JSON.stringify(resolved))}::jsonb)`,
      );
    }
    if (table !== 'alliances' && table !== 'corporations') {
      psql(
        `select setval(pg_get_serial_sequence('${table}', 'id'), (select max(id) from ${table}))`,
      );
    }
  }
}

/** The plain token of a dataset token id. */
export function datasetToken(id: number): string {
  const path = new URL('../../tests/fixtures/dataset.json', import.meta.url);
  const dataset = JSON.parse(readFileSync(path, 'utf8')) as {
    personal_access_tokens: Record<string, string | number>[];
  };
  const token = dataset.personal_access_tokens.find((row) => row.id === id);
  if (!token) throw new Error(`no dataset token ${id}`);
  // The plain text the dataset keeps beside the stored hash.
  return String(token['_plain']);
}

/** Collects uncaught page errors and console errors (Pest's
 * assertNoJavascriptErrors). The share page opens `telescope://` links,
 * which a browser without the app refuses with a console line; that one
 * is expected. */
export function trackJavascriptErrors(page: Page): () => void {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    const deepLink = message.text().includes('telescope://');
    if (message.type() === 'error' && !deepLink) {
      errors.push(message.text());
    }
  });
  return () => expect(errors, 'javascript errors').toEqual([]);
}

/** Pest's `assertSee`. */
export async function assertSee(page: Page, text: string): Promise<void> {
  await expect(page.locator('body')).toContainText(text);
}
