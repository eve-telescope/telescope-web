#!/usr/bin/env python3
"""Records the golden fixtures from the Laravel reference.

Runs against `run.sh start` (http://127.0.0.1:8318). Before every
scenario the database is reset to tests/fixtures/dataset.json, then the
scenario's request is sent the way the desktop clients send it, and its
answer (status, the headers that matter, the body) and the rows of the
tables it changes are written to tests/fixtures/golden/api/<name>.json.
The Inertia pages are recorded as their page objects (X-Inertia).

    python3 -I scripts/laravel-reference/record.py [name-prefix]

Stdlib only. Nothing here is a secret: the tokens are the dataset's.
"""

import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BASE = "http://127.0.0.1:8318"
ORIGIN = "https://eve-telescope.com"
OUT = os.path.join(REPO, "tests", "fixtures", "golden", "api")
DATASET = json.load(open(os.path.join(REPO, "tests", "fixtures", "dataset.json"), encoding="utf-8"))
TOKENS = {row["id"]: row["_plain"] for row in DATASET["personal_access_tokens"]}
USER_AGENT = "Telescope | https://eve-telescope.com"

# The columns of every table, in order, as the snapshots list them.
COLUMNS = {
    "alliances": ["id", "name", "ticker", "last_updated", "created_at", "updated_at"],
    "corporations": ["id", "name", "ticker", "alliance_id", "member_count", "last_updated", "created_at", "updated_at"],
    "users": ["id", "character_id", "character_name", "character_owner_hash", "corporation_id", "alliance_id", "last_active_at", "created_at", "updated_at"],
    "personal_access_tokens": ["id", "tokenable_type", "tokenable_id", "name", "token", "abilities", "last_used_at", "expires_at", "created_at", "updated_at"],
    "intel_networks": ["id", "name", "slug", "owner_id", "created_at", "updated_at"],
    "network_access": ["id", "intel_network_id", "accessible_type", "accessible_id", "permission", "is_owner", "expires_at", "created_at", "updated_at"],
    "intel_entries": ["id", "intel_network_id", "entity_type", "entity_id", "entity_name", "color", "label", "notes", "added_by", "created_at", "updated_at"],
    "network_scans": ["id", "intel_network_id", "user_id", "scan_type", "raw_text", "solar_system", "created_at", "updated_at"],
    "shares": ["id", "code", "pilots", "pilot_count", "views", "created_at", "updated_at"],
}
TIMESTAMPS = {"last_updated", "last_active_at", "last_used_at", "expires_at", "created_at", "updated_at"}
KEPT_HEADERS = {
    "content-type", "location", "allow", "vary", "x-inertia-location", "www-authenticate",
    "access-control-allow-origin", "access-control-allow-methods", "access-control-allow-headers",
    "access-control-max-age", "access-control-allow-credentials", "access-control-expose-headers",
}


def sql_value(value):
    if value is None:
        return "NULL"
    if isinstance(value, bool):
        return "1" if value else "0"
    if isinstance(value, (int, float)):
        return str(value)
    if isinstance(value, (list, dict)):
        value = json.dumps(value, ensure_ascii=False)
    return "'" + value.replace("\\", "\\\\").replace("'", "''") + "'"


def reset_sql():
    lines = ["SET FOREIGN_KEY_CHECKS=0;"]
    for table in list(COLUMNS) + ["sessions"]:
        lines.append(f"TRUNCATE {table};")
    for table, columns in COLUMNS.items():
        for row in DATASET[table]:
            values = ", ".join(sql_value(row[column]) for column in columns)
            lines.append(f"INSERT INTO {table} ({', '.join(columns)}) VALUES ({values});")
    lines.append("SET FOREIGN_KEY_CHECKS=1;")
    return "\n".join(lines)


RESET = reset_sql()


def mysql(sql):
    result = subprocess.run(
        ["docker", "exec", "-i", "telescope-laravel-mysql", "mysql", "-uroot", "-ptelescope",
         "--default-character-set=utf8mb4", "-N", "-B", "-r", "laravel"],
        input=sql.encode("utf-8"), capture_output=True, check=True,
    )
    return result.stdout.decode("utf-8")


def snapshot(table):
    columns = COLUMNS[table]
    parts = []
    for column in columns:
        if column in TIMESTAMPS:
            parts.append(f"'{column}', DATE_FORMAT({column}, '%Y-%m-%d %H:%i:%s')")
        elif column == "is_owner":
            parts.append(f"'{column}', IF({column}, CAST('true' AS JSON), CAST('false' AS JSON))")
        else:
            parts.append(f"'{column}', {column}")
    output = mysql(f"SELECT JSON_OBJECT({', '.join(parts)}) FROM {table} ORDER BY id;")
    return [json.loads(line, object_pairs_hook=dict) for line in output.splitlines() if line.strip()]


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


OPENER = urllib.request.build_opener(NoRedirect)


def send(method, path, headers, body):
    request = urllib.request.Request(BASE + path, data=body, method=method, headers=headers)
    try:
        response = OPENER.open(request, timeout=30)
        status, response_headers, data = response.status, response.headers, response.read()
    except urllib.error.HTTPError as error:
        status, response_headers, data = error.code, error.headers, error.read()
    return status, response_headers, data


def localize(value):
    """N2: the reference's own origin becomes the production one, in text
    and in every string of parsed JSON (Laravel escapes `/` as `\\/`)."""
    if isinstance(value, str):
        return value.replace(BASE, ORIGIN)
    if isinstance(value, list):
        return [localize(item) for item in value]
    if isinstance(value, dict):
        return {key: localize(item) for key, item in value.items()}
    return value


def inertia_version():
    status, _, data = send("GET", "/", {"User-Agent": USER_AGENT}, None)
    page = re.search(rb'<script data-page="app" type="application/json">(.*?)</script>', data, re.S)
    if page:
        return json.loads(page.group(1))["version"]
    page = re.search(rb'data-page="([^"]*)"', data)
    import html
    return json.loads(html.unescape(page.group(1).decode()))["version"]


VERSION = None
SCENARIOS = []


def scenario(name, method, path, token=None, body=None, form=None, raw=None, rows=(),
             accept="application/json", headers=None, inertia=False, json_type=True):
    SCENARIOS.append(dict(name=name, method=method, path=path, token=token, body=body, form=form,
                          raw=raw, rows=list(rows), accept=accept, headers=headers or {},
                          inertia=inertia, json_type=json_type))


def page(name, path, rows=()):
    scenario(name, "GET", path, accept=None, inertia=True, rows=rows)


def run(item):
    headers = {"User-Agent": USER_AGENT}
    body = None
    if item["accept"]:
        headers["Accept"] = item["accept"]
    if item["token"] is not None:
        token = TOKENS.get(item["token"], item["token"])
        headers["Authorization"] = f"Bearer {token}"
    if item["inertia"]:
        headers["X-Inertia"] = "true"
        headers["X-Inertia-Version"] = VERSION
        headers["X-Requested-With"] = "XMLHttpRequest"
    if item["form"] is not None:
        headers["Content-Type"] = "application/x-www-form-urlencoded"
        body = urllib.parse.urlencode(item["form"]).encode()
    elif item["raw"] is not None:
        body = item["raw"].encode()
    elif item["body"] is not None:
        headers["Content-Type"] = "application/json"
        body = json.dumps(item["body"], ensure_ascii=False).encode("utf-8")
    elif item["json_type"] and item["accept"] == "application/json":
        # The desktop client sends a JSON content type even without a body.
        headers["Content-Type"] = "application/json"
    headers.update(item["headers"])

    mysql(RESET)
    started = time.time()
    status, response_headers, data = send(item["method"], item["path"], headers, body)
    finished = time.time()

    kept = {}
    for key in response_headers.keys():
        lower = key.lower()
        if lower in KEPT_HEADERS:
            kept[lower] = localize(response_headers.get(key))
        if lower == "set-cookie":
            kept.setdefault("set-cookie", [])
    if "set-cookie" in kept:
        kept["set-cookie"] = sorted(
            cookie.split("=", 1)[0] for cookie in response_headers.get_all("Set-Cookie") or []
        )
    text = data.decode("utf-8", errors="replace")
    response = {"status": status, "headers": kept}
    try:
        response["json"] = localize(json.loads(text, object_pairs_hook=dict)) if text.strip() else None
        if response["json"] is None:
            response["text"] = text
    except json.JSONDecodeError:
        response["text"] = localize(text)
    request = {key: item[key] for key in ("method", "path", "token", "body", "form", "raw", "accept", "headers", "inertia", "json_type")}
    fixture = {
        "name": item["name"],
        "request": request,
        "response": response,
        "rows": {table: snapshot(table) for table in item["rows"]},
        "started_at": int(started),
        "finished_at": int(finished) + 1,
    }
    with open(os.path.join(OUT, item["name"] + ".json"), "w", encoding="utf-8") as file:
        json.dump(fixture, file, indent=2, ensure_ascii=False)
        file.write("\n")
    return status


# ---------------------------------------------------------------------------
# The scenarios. Tokens: 1 Owner Pilot, 2 Member Pilot, 3 Viewer Pilot,
# 4 Outsider Pilot, 5 Corp Mate, 6 Lapsed Pilot, 7 Ally Pilot, 8 Owner's
# second token (CLI), 9 Owner's expired token, 10 José Ørsted.
# ---------------------------------------------------------------------------

def define():
    # Pages.
    page("page_home", "/")
    page("page_share", "/s/ABCDEFGH", rows=["shares"])
    page("page_share_lower_case", "/s/abcdefgh", rows=["shares"])
    page("page_share_unicode", "/s/Unic0de1")
    page("page_share_many", "/s/zyxW9876")
    page("page_share_missing", "/s/NOPE0000")
    scenario("page_share_trailing_slash", "GET", "/s/ABCDEFGH/", accept=None, inertia=True)
    scenario("page_home_stale_version", "GET", "/", accept=None, headers={"X-Inertia": "true", "X-Inertia-Version": "stale"})

    # Web routes.
    scenario("eve_redirect", "GET", "/eve", accept=None)
    scenario("eve_redirect_desktop", "GET", "/eve?desktop", accept=None)
    scenario("eve_callback_without_state", "GET", "/eve/callback?code=abc&state=xyz", accept=None)
    scenario("csrf_cookie", "GET", "/sanctum/csrf-cookie", accept=None)
    scenario("up_page", "GET", "/up", accept=None)
    scenario("up_json", "GET", "/up")
    scenario("storage_get", "GET", "/storage/file.txt", accept=None)
    scenario("storage_put", "PUT", "/storage/file.txt", accept=None, raw="x")
    scenario("unknown_page", "GET", "/nope", accept=None)
    scenario("unknown_api_json", "GET", "/api/nope")
    scenario("unknown_api_html", "GET", "/api/nope", accept=None)

    # The token guard.
    scenario("user_show", "GET", "/api/user", token=1, rows=["personal_access_tokens"])
    scenario("user_show_second_token", "GET", "/api/user", token=8, rows=["personal_access_tokens"])
    scenario("user_show_unicode", "GET", "/api/user", token=10)
    scenario("user_guest_json", "GET", "/api/user")
    scenario("user_guest_html", "GET", "/api/user", accept=None)
    scenario("user_bad_token", "GET", "/api/user", token="1|not-the-token")
    scenario("user_expired_token", "GET", "/api/user", token=9, rows=["personal_access_tokens"])
    scenario("user_token_without_id", "GET", "/api/user", token=TOKENS[1].split("|", 1)[1])
    scenario("user_token_with_wrong_id", "GET", "/api/user", token="2|" + TOKENS[1].split("|", 1)[1])
    scenario("user_token_unknown_id", "GET", "/api/user", token="999|" + TOKENS[1].split("|", 1)[1])
    scenario("user_trailing_slash", "GET", "/api/user/", token=1)

    # Tokens.
    scenario("tokens_create", "POST", "/api/tokens", token=1, body={"name": "Laptop"}, rows=["personal_access_tokens"])
    scenario("tokens_create_invalid", "POST", "/api/tokens", token=1, body={})
    scenario("tokens_create_long", "POST", "/api/tokens", token=1, body={"name": "x" * 256})
    scenario("tokens_create_number", "POST", "/api/tokens", token=1, body={"name": 42})
    scenario("tokens_revoke_own", "DELETE", "/api/tokens/8", token=1, rows=["personal_access_tokens"])
    scenario("tokens_revoke_current", "DELETE", "/api/tokens/1", token=1, rows=["personal_access_tokens"])
    scenario("tokens_revoke_foreign", "DELETE", "/api/tokens/2", token=1, rows=["personal_access_tokens"])
    scenario("tokens_revoke_missing", "DELETE", "/api/tokens/999", token=1)
    scenario("tokens_revoke_non_numeric", "DELETE", "/api/tokens/abc", token=1)

    # Networks.
    for token, who in [(1, "owner"), (2, "member"), (3, "viewer"), (4, "outsider"), (5, "corp_mate"), (6, "lapsed"), (7, "ally"), (10, "stranger")]:
        scenario(f"networks_index_{who}", "GET", "/api/networks", token=token)
    scenario("networks_store", "POST", "/api/networks", token=3, body={"name": "Delta Recon"}, rows=["intel_networks", "network_access"])
    scenario("networks_store_unicode", "POST", "/api/networks", token=10, body={"name": "  Ørsted & Søn's Ops!  "}, rows=["intel_networks", "network_access"])
    scenario("networks_store_symbols", "POST", "/api/networks", token=1, body={"name": "!!!"}, rows=["intel_networks"])
    scenario("networks_store_empty", "POST", "/api/networks", token=1, body={"name": ""})
    scenario("networks_store_missing", "POST", "/api/networks", token=1, body={})
    scenario("networks_store_long", "POST", "/api/networks", token=1, body={"name": "x" * 256})
    scenario("networks_store_number", "POST", "/api/networks", token=1, body={"name": 123})
    scenario("networks_store_no_accept", "POST", "/api/networks", token=1, body={}, accept=None)
    for token, who in [(1, "owner"), (3, "viewer"), (5, "corp_mate"), (7, "ally")]:
        scenario(f"networks_show_{who}", "GET", "/api/networks/1", token=token)
    scenario("networks_show_bravo", "GET", "/api/networks/2", token=7)
    scenario("networks_show_outsider", "GET", "/api/networks/1", token=4)
    scenario("networks_show_lapsed", "GET", "/api/networks/1", token=6)
    scenario("networks_show_missing", "GET", "/api/networks/999", token=1)
    scenario("networks_show_letters", "GET", "/api/networks/abc", token=1)
    scenario("networks_show_trailing_letters", "GET", "/api/networks/1abc", token=1)
    scenario("networks_show_missing_html", "GET", "/api/networks/999", token=1, accept=None)
    scenario("networks_show_trailing_slash", "GET", "/api/networks/1/", token=1)
    scenario("networks_update", "PUT", "/api/networks/1", token=1, body={"name": "Alpha Intel Renamed"}, rows=["intel_networks"])
    scenario("networks_update_same_name", "PUT", "/api/networks/1", token=1, body={"name": "Alpha Intel"}, rows=["intel_networks"])
    scenario("networks_update_by_manager", "PUT", "/api/networks/1", token=7, body={"name": "Ally Rename"}, rows=["intel_networks"])
    scenario("networks_update_by_member", "PUT", "/api/networks/1", token=2, body={"name": "Hacked"})
    scenario("networks_update_invalid", "PUT", "/api/networks/1", token=1, body={"name": ""})
    scenario("networks_update_forbidden_and_invalid", "PUT", "/api/networks/1", token=2, body={})
    scenario("networks_destroy", "DELETE", "/api/networks/1", token=1, rows=["intel_networks", "network_access", "intel_entries", "network_scans"])
    scenario("networks_destroy_by_manager", "DELETE", "/api/networks/1", token=7)
    scenario("networks_destroy_by_outsider", "DELETE", "/api/networks/1", token=4)
    scenario("networks_guest", "GET", "/api/networks")
    scenario("networks_patch", "PATCH", "/api/networks/1", token=1, body={"name": "x"})

    # Access.
    scenario("access_index", "GET", "/api/networks/1/access", token=1)
    scenario("access_index_bravo", "GET", "/api/networks/2/access", token=4)
    scenario("access_index_by_member", "GET", "/api/networks/1/access", token=2)
    grant = {"accessible_type": "character", "accessible_id": 90000004, "accessible_name": "Outsider Pilot", "permission": "viewer"}
    scenario("access_store_character", "POST", "/api/networks/1/access", token=1, body=grant, rows=["network_access"])
    scenario("access_store_corporation", "POST", "/api/networks/1/access", token=1, body={"accessible_type": "corporation", "accessible_id": 98000003, "accessible_name": "Solo Corp", "permission": "member"}, rows=["network_access"])
    scenario("access_store_alliance", "POST", "/api/networks/1/access", token=1, body={"accessible_type": "alliance", "accessible_id": 99000001, "accessible_name": "Goonswarm", "permission": "manager"}, rows=["network_access"])
    scenario("access_store_unknown_entity", "POST", "/api/networks/1/access", token=1, body={"accessible_type": "character", "accessible_id": "90000555", "accessible_name": "Nobody", "permission": "member"}, rows=["network_access"])
    scenario("access_store_corporation_without_alliance", "POST", "/api/networks/2/access", token=4, body={"accessible_type": "corporation", "accessible_id": 98000005, "accessible_name": "Pure", "permission": "viewer"}, rows=["network_access"])
    scenario("access_store_with_expiry", "POST", "/api/networks/1/access", token=1, body=dict(grant, expires_at="2027-06-01 12:00:00"), rows=["network_access"])
    scenario("access_store_with_date_expiry", "POST", "/api/networks/1/access", token=1, body=dict(grant, expires_at="2027-06-01"), rows=["network_access"])
    scenario("access_store_expiry_with_offset", "POST", "/api/networks/1/access", token=1, body=dict(grant, expires_at="2027-06-01T10:00:00+02:00"), rows=["network_access"])
    scenario("access_store_past_expiry", "POST", "/api/networks/1/access", token=1, body=dict(grant, expires_at="2026-01-01"))
    scenario("access_store_words_expiry", "POST", "/api/networks/1/access", token=1, body=dict(grant, expires_at="tomorrow"), rows=["network_access"])
    scenario("access_store_bad_expiry", "POST", "/api/networks/1/access", token=1, body=dict(grant, expires_at="not a date"))
    scenario("access_store_upsert", "POST", "/api/networks/1/access", token=1, body={"accessible_type": "character", "accessible_id": 90000002, "accessible_name": "Member Pilot", "permission": "manager"}, rows=["network_access"])
    scenario("access_store_upsert_clears_expiry", "POST", "/api/networks/1/access", token=1, body={"accessible_type": "alliance", "accessible_id": 99000002, "accessible_name": "TEST", "permission": "viewer"}, rows=["network_access"])
    scenario("access_store_owner_row", "POST", "/api/networks/1/access", token=7, body={"accessible_type": "character", "accessible_id": 90000001, "accessible_name": "Owner Pilot", "permission": "viewer", "expires_at": "2027-01-01"}, rows=["network_access"])
    scenario("access_store_invalid", "POST", "/api/networks/1/access", token=1, body={})
    scenario("access_store_bad_values", "POST", "/api/networks/1/access", token=1, body={"accessible_type": "ship", "accessible_id": "x1", "accessible_name": ["a"], "permission": "admin"})
    scenario("access_store_by_member", "POST", "/api/networks/1/access", token=2, body=grant)
    scenario("access_update", "PUT", "/api/networks/1/access/3", token=1, body={"permission": "member"}, rows=["network_access"])
    scenario("access_update_expiry", "PUT", "/api/networks/1/access/3", token=1, body={"expires_at": "2027-03-01 08:00:00"}, rows=["network_access"])
    scenario("access_update_clear_expiry", "PUT", "/api/networks/1/access/6", token=1, body={"expires_at": None}, rows=["network_access"])
    scenario("access_update_nothing", "PUT", "/api/networks/1/access/3", token=1, body={}, rows=["network_access"])
    scenario("access_update_same", "PUT", "/api/networks/1/access/3", token=1, body={"permission": "viewer"}, rows=["network_access"])
    scenario("access_update_owner", "PUT", "/api/networks/1/access/1", token=1, body={"permission": "viewer"})
    scenario("access_update_invalid", "PUT", "/api/networks/1/access/3", token=1, body={"permission": "admin", "expires_at": "2020-01-01"})
    scenario("access_update_null_permission", "PUT", "/api/networks/1/access/3", token=1, body={"permission": None})
    scenario("access_update_foreign", "PUT", "/api/networks/1/access/8", token=1, body={"permission": "viewer"}, rows=["network_access"])
    scenario("access_update_missing", "PUT", "/api/networks/1/access/999", token=1, body={"permission": "viewer"})
    scenario("access_update_by_member", "PUT", "/api/networks/1/access/3", token=2, body={"permission": "manager"})
    scenario("access_destroy", "DELETE", "/api/networks/1/access/3", token=1, rows=["network_access"])
    scenario("access_destroy_owner", "DELETE", "/api/networks/1/access/1", token=7)
    scenario("access_destroy_foreign", "DELETE", "/api/networks/1/access/8", token=1, rows=["network_access"])
    scenario("access_destroy_by_member", "DELETE", "/api/networks/1/access/3", token=2)

    # Entries.
    scenario("entries_index", "GET", "/api/networks/1/entries", token=3)
    scenario("entries_index_bravo", "GET", "/api/networks/2/entries", token=7)
    scenario("entries_index_outsider", "GET", "/api/networks/1/entries", token=4)
    new = {"entity_type": "character", "entity_id": 90001010, "entity_name": "New Target", "color": "#FF3B3B", "label": "HOSTILE", "notes": None}
    scenario("entries_store", "POST", "/api/networks/1/entries", token=2, body=new, rows=["intel_entries"])
    scenario("entries_store_note_only", "POST", "/api/networks/1/entries", token=2, body={"entity_type": "character", "entity_id": 90001011, "entity_name": "Noted Guy", "notes": "Seen in Jita"}, rows=["intel_entries"])
    scenario("entries_store_empty_strings", "POST", "/api/networks/1/entries", token=2, body={"entity_type": "alliance", "entity_id": 99000555, "entity_name": "Some Alliance", "color": "", "label": "", "notes": "  "}, rows=["intel_entries"])
    scenario("entries_store_upsert", "POST", "/api/networks/1/entries", token=2, body={"entity_type": "character", "entity_id": 90001001, "entity_name": "Hostile Hank", "color": "#00FF00", "label": "HOSTILE | SPY"}, rows=["intel_entries"])
    scenario("entries_store_same_entity_other_type", "POST", "/api/networks/1/entries", token=2, body={"entity_type": "corporation", "entity_id": 90001001, "entity_name": "Hank Corp"}, rows=["intel_entries"])
    scenario("entries_store_string_id", "POST", "/api/networks/1/entries", token=2, body={"entity_type": "character", "entity_id": "90001012", "entity_name": "Text Id"}, rows=["intel_entries"])
    scenario("entries_store_by_viewer", "POST", "/api/networks/1/entries", token=3, body=new)
    scenario("entries_store_by_outsider", "POST", "/api/networks/1/entries", token=4, body=new)
    scenario("entries_store_invalid", "POST", "/api/networks/1/entries", token=2, body={"entity_type": "ship", "entity_id": 1.5, "color": "red", "label": "x" * 256, "notes": "y" * 10001})
    scenario("entries_store_missing", "POST", "/api/networks/1/entries", token=2, body={})
    scenario("entries_store_unicode", "POST", "/api/networks/1/entries", token=2, body={"entity_type": "character", "entity_id": 90001013, "entity_name": "José Ørsted", "notes": "Ünïcode ✓ / slash"}, rows=["intel_entries"])
    scenario("entries_update", "PUT", "/api/networks/1/entries/1", token=2, body={"label": "FRIENDLY", "notes": "Actually a friend"}, rows=["intel_entries"])
    scenario("entries_update_name_and_color", "PUT", "/api/networks/1/entries/2", token=1, body={"entity_name": "Spy Corp [SPY]", "color": "#123ABC"}, rows=["intel_entries"])
    scenario("entries_update_same", "PUT", "/api/networks/1/entries/1", token=2, body={"label": "HOSTILE"}, rows=["intel_entries"])
    scenario("entries_update_clear_notes", "PUT", "/api/networks/1/entries/2", token=2, body={"notes": None}, rows=["intel_entries"])
    scenario("entries_update_clear_label", "PUT", "/api/networks/1/entries/1", token=2, body={"label": ""}, rows=["intel_entries"])
    scenario("entries_update_v05_client", "PUT", "/api/networks/1/entries/2", token=2, body={"entity_type": "corporation", "entity_id": 98000777, "entity_name": "Spy Corp", "color": "", "label": "SPY", "notes": "Watch their alts"}, rows=["intel_entries"])
    scenario("entries_update_invalid", "PUT", "/api/networks/1/entries/1", token=2, body={"color": "#GGGGGG", "entity_name": 5})
    scenario("entries_update_by_viewer", "PUT", "/api/networks/1/entries/1", token=3, body={"label": "X"})
    scenario("entries_update_foreign", "PUT", "/api/networks/1/entries/6", token=2, body={"label": "PWNED"}, rows=["intel_entries"])
    scenario("entries_update_missing", "PUT", "/api/networks/1/entries/999", token=2, body={"label": "X"})
    scenario("entries_patch", "PATCH", "/api/networks/1/entries/1", token=2, body={"label": "X"})
    scenario("entries_patch_html", "PATCH", "/api/networks/1/entries/1", token=2, body={"label": "X"}, accept=None)
    scenario("entries_destroy", "DELETE", "/api/networks/1/entries/2", token=2, rows=["intel_entries"])
    scenario("entries_destroy_by_viewer", "DELETE", "/api/networks/1/entries/2", token=3)
    scenario("entries_destroy_foreign", "DELETE", "/api/networks/1/entries/6", token=2, rows=["intel_entries"])
    scenario("entries_destroy_missing", "DELETE", "/api/networks/1/entries/999", token=2)

    # Lookup.
    scenario("lookup_viewer", "GET", "/api/intel/lookup?entity_ids[]=90001001&entity_ids[]=98000777&entity_ids[]=90001002", token=3)
    scenario("lookup_ally", "GET", "/api/intel/lookup?entity_ids[]=90001001&entity_ids[]=90001002&entity_ids[]=90001003", token=7)
    scenario("lookup_member", "GET", "/api/intel/lookup?entity_ids%5B0%5D=90001003&entity_ids%5B1%5D=99000777", token=2)
    scenario("lookup_lapsed", "GET", "/api/intel/lookup?entity_ids[]=90001001", token=6)
    scenario("lookup_missing", "GET", "/api/intel/lookup", token=3)
    scenario("lookup_not_array", "GET", "/api/intel/lookup?entity_ids=90001001", token=3)
    scenario("lookup_invalid", "GET", "/api/intel/lookup?entity_ids[]=abc&entity_ids[]=&entity_ids[]=5", token=3)

    # Scans.
    scenario("scans_index", "GET", "/api/networks/1/scans", token=3)
    scenario("scans_index_page_2", "GET", "/api/networks/1/scans?page=2", token=3)
    scenario("scans_index_page_99", "GET", "/api/networks/1/scans?page=99", token=3)
    scenario("scans_index_page_letters", "GET", "/api/networks/1/scans?page=abc", token=3)
    scenario("scans_index_page_zero", "GET", "/api/networks/1/scans?page=0&other=1", token=3)
    scenario("scans_index_bravo", "GET", "/api/networks/2/scans", token=7)
    scenario("scans_index_empty", "GET", "/api/networks/3/scans", token=2)
    scenario("scans_index_outsider", "GET", "/api/networks/1/scans", token=4)
    scenario("scans_store_local", "POST", "/api/networks/1/scans", token=2, body={"scan_type": "local", "raw_text": "Pilot One\nPilot Two\nPilot Three"}, rows=["network_scans"])
    scenario("scans_store_dscan", "POST", "/api/networks/1/scans", token=1, body={"scan_type": "dscan", "raw_text": "12345\tShip Name\tRifter\t100 km", "solar_system": "Jita"}, rows=["network_scans"])
    scenario("scans_store_by_viewer", "POST", "/api/networks/1/scans", token=3, body={"scan_type": "local", "raw_text": "Pilot One"})
    scenario("scans_store_invalid", "POST", "/api/networks/1/scans", token=2, body={"scan_type": "invalid", "raw_text": "test", "solar_system": "x" * 256})
    scenario("scans_store_missing_text", "POST", "/api/networks/1/scans", token=2, body={"scan_type": "local"})
    scenario("scans_store_blank_text", "POST", "/api/networks/1/scans", token=2, body={"scan_type": "local", "raw_text": " \n "})
    scenario("scans_show", "GET", "/api/networks/1/scans/1", token=3)
    scenario("scans_show_foreign", "GET", "/api/networks/1/scans/26", token=3)
    scenario("scans_show_missing", "GET", "/api/networks/1/scans/999", token=3)
    scenario("scans_show_outsider", "GET", "/api/networks/1/scans/1", token=4)
    scenario("scans_show_outsider_foreign", "GET", "/api/networks/1/scans/26", token=4)

    # Search.
    search = [
        ("search_character", "query=Owner&category=character"),
        ("search_character_case", "query=oWNER+pilot&category=character"),
        ("search_character_accent", "query=jose&category=character"),
        ("search_corporation_name", "query=Karma&category=corporation"),
        ("search_corporation_ticker", "query=TSTC&category=corporation"),
        ("search_alliance_name", "query=Goon&category=alliance"),
        ("search_alliance_accent", "query=federation&category=alliance"),
        ("search_all_categories", "query=Test"),
        ("search_wildcard_percent", "query=%25%25&category=corporation"),
        ("search_wildcard_underscore", "query=Pure_&category=corporation"),
        ("search_no_local_match", "query=Nobody+Here&category=character"),
        ("search_missing_query", ""),
        ("search_short_query", "query=a"),
        ("search_invalid_category", "query=test&category=invalid"),
        ("search_array_query", "query[]=ab"),
    ]
    for name, query in search:
        scenario(name, "GET", "/api/search" + ("?" + query if query else ""), token=1)

    # Shares.
    scenario("share_store", "POST", "/api/share", body={"pilots": ["Pilot One", " Pilot Two ", "Pilot One", "0", "pilot one", "José Ørsted"]}, rows=["shares"])
    scenario("share_store_empty", "POST", "/api/share", body={"pilots": []})
    scenario("share_store_missing", "POST", "/api/share", body={})
    scenario("share_store_not_array", "POST", "/api/share", body={"pilots": "Pilot One"})
    scenario("share_store_bad_items", "POST", "/api/share", body={"pilots": ["Fine", None, "x" * 101, 5, ""]})
    scenario("share_store_too_many", "POST", "/api/share", body={"pilots": [f"P{i}" for i in range(501)]})
    scenario("share_store_no_accept", "POST", "/api/share", body={}, accept=None)
    scenario("share_fetch", "GET", "/api/share/ABCDEFGH", rows=["shares"])
    scenario("share_fetch_lower_case", "GET", "/api/share/abcdefgh")
    scenario("share_fetch_unicode", "GET", "/api/share/Unic0de1")
    scenario("share_fetch_missing", "GET", "/api/share/NOPE0000")
    scenario("share_fetch_missing_html", "GET", "/api/share/NOPE0000", accept=None)

    # Channel auth.
    def auth(name, token, channel, socket="123.456", method="POST", accept="application/json"):
        if method == "GET":
            query = urllib.parse.urlencode({"socket_id": socket, "channel_name": channel})
            scenario(name, "GET", "/broadcasting/auth?" + query, token=token, accept=accept, json_type=False)
        else:
            form = {}
            if socket is not None:
                form["socket_id"] = socket
            if channel is not None:
                form["channel_name"] = channel
            scenario(name, "POST", "/broadcasting/auth", token=token, form=form, accept=accept)

    auth("bauth_owner", 1, "private-intel-network.1")
    auth("bauth_viewer", 3, "private-intel-network.1")
    auth("bauth_corp_mate", 5, "private-intel-network.1")
    auth("bauth_ally_bravo", 7, "private-intel-network.2")
    auth("bauth_outsider", 4, "private-intel-network.1")
    auth("bauth_lapsed", 6, "private-intel-network.1")
    auth("bauth_missing_network", 1, "private-intel-network.999")
    auth("bauth_non_numeric_network", 1, "private-intel-network.abc")
    auth("bauth_unknown_channel", 1, "private-somewhere.1")
    auth("bauth_public_channel", 1, "intel-network.1")
    auth("bauth_presence_channel", 1, "presence-intel-network.1")
    auth("bauth_missing_channel", 1, None)
    auth("bauth_missing_socket", 1, "private-intel-network.1", socket=None)
    auth("bauth_bad_socket", 1, "private-intel-network.1", socket="abc")
    auth("bauth_get", 1, "private-intel-network.1", method="GET")
    auth("bauth_guest", None, "private-intel-network.1")
    auth("bauth_guest_html", None, "private-intel-network.1", accept=None)
    auth("bauth_no_accept", 1, "private-intel-network.1", accept=None)
    scenario("bauth_json_body", "POST", "/broadcasting/auth", token=1, body={"socket_id": "123.456", "channel_name": "private-intel-network.1"})

    # CORS.
    scenario("cors_preflight_auth", "OPTIONS", "/broadcasting/auth", accept=None, json_type=False, headers={"Origin": "tauri://localhost", "Access-Control-Request-Method": "POST", "Access-Control-Request-Headers": "authorization, content-type"})
    scenario("cors_preflight_api", "OPTIONS", "/api/networks", accept=None, json_type=False, headers={"Origin": "http://tauri.localhost", "Access-Control-Request-Method": "get"})
    scenario("cors_preflight_unknown", "OPTIONS", "/api/nope/deeper", accept=None, json_type=False, headers={"Origin": "http://x.test", "Access-Control-Request-Method": "DELETE", "Access-Control-Request-Headers": "x-custom"})
    scenario("cors_simple", "GET", "/api/share/ABCDEFGH", headers={"Origin": "tauri://localhost"})
    scenario("cors_auth_answer", "POST", "/broadcasting/auth", token=1, form={"socket_id": "1.2", "channel_name": "private-intel-network.1"}, headers={"Origin": "tauri://localhost"})
    scenario("cors_error_answer", "GET", "/api/networks", headers={"Origin": "tauri://localhost"})


def main():
    global VERSION
    if sys.argv[1:] == ["--print-reset"]:
        # The dataset as SQL, for pusher.mjs.
        print(RESET)
        return
    os.makedirs(OUT, exist_ok=True)
    prefix = sys.argv[1] if len(sys.argv) > 1 else ""
    VERSION = inertia_version()
    define()
    names = [item["name"] for item in SCENARIOS]
    duplicates = {name for name in names if names.count(name) > 1}
    assert not duplicates, duplicates
    for item in SCENARIOS:
        if item["name"].startswith(prefix):
            status = run(item)
            print(f"{status} {item['name']}")
    meta_path = os.path.join(REPO, "tests", "fixtures", "golden", "meta.json")
    github = None
    try:
        page = json.load(open(os.path.join(OUT, "page_home.json"), encoding="utf-8"))
        github = page["response"]["json"]["props"]["appVersion"]
    except (OSError, KeyError, TypeError):
        pass
    with open(meta_path, "w", encoding="utf-8") as file:
        json.dump({"inertia_version": VERSION, "app_version": github, "laravel_commit": "247ffd1"}, file, indent=2)
        file.write("\n")


if __name__ == "__main__":
    main()
