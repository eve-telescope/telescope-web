<?php

use App\Models\Alliance;
use App\Models\Corporation;
use App\Models\IntelNetwork;
use App\Models\NetworkAccess;
use App\Models\User;

beforeEach(function (): void {
    $this->alliance = Alliance::create([
        'id' => 99000001,
        'name' => 'Test Alliance',
        'ticker' => 'TSTA',
    ]);

    $this->corporation = Corporation::create([
        'id' => 98000001,
        'name' => 'Test Corp',
        'ticker' => 'TSTC',
        'alliance_id' => $this->alliance->id,
    ]);

    $this->owner = User::factory()->create([
        'corporation_id' => $this->corporation->id,
        'alliance_id' => $this->alliance->id,
    ]);

    $this->network = IntelNetwork::create([
        'name' => 'Test Network',
        'owner_id' => $this->owner->id,
    ]);

    $this->network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $this->owner->character_id,
        'permission' => 'manager',
        'is_owner' => true,
    ]);
});

// ---------------------------------------------------------------------------
// POST /api/networks/{network}/access — store
// ---------------------------------------------------------------------------

it('can grant character access and returns correct JSON shape', function (): void {
    $target = User::factory()->create([
        'corporation_id' => $this->corporation->id,
        'alliance_id' => $this->alliance->id,
    ]);

    $response = $this->actingAs($this->owner)
        ->postJson("/api/networks/{$this->network->id}/access", [
            'accessible_type' => 'character',
            'accessible_id' => $target->character_id,
            'accessible_name' => $target->character_name,
            'permission' => 'viewer',
        ]);

    $response->assertStatus(201)
        ->assertJsonStructure([
            'id',
            'accessible_type',
            'accessible_id',
            'permission',
            'is_owner',
            'expires_at',
            'entity' => [
                'id',
                'name',
                'type',
                'ticker',
                'corporation',
                'alliance',
            ],
        ]);

    // accessible_type must be a simple string, NOT a PHP class name
    $data = $response->json();
    expect($data['accessible_type'])->toBe('character');
    expect($data['accessible_id'])->toBe($target->character_id);
    expect($data['permission'])->toBe('viewer');
    expect($data['is_owner'])->toBe(false);
    expect($data['entity']['type'])->toBe('character');
    expect($data['entity']['name'])->toBe($target->character_name);
    expect($data['entity']['id'])->toBe($target->character_id);
});

it('can grant corporation access and returns correct JSON shape', function (): void {
    $response = $this->actingAs($this->owner)
        ->postJson("/api/networks/{$this->network->id}/access", [
            'accessible_type' => 'corporation',
            'accessible_id' => $this->corporation->id,
            'accessible_name' => $this->corporation->name,
            'permission' => 'member',
        ]);

    $response->assertStatus(201);

    $data = $response->json();
    expect($data['accessible_type'])->toBe('corporation');
    expect($data['accessible_id'])->toBe($this->corporation->id);
    expect($data['permission'])->toBe('member');
    expect($data['is_owner'])->toBe(false);
    expect($data['entity']['type'])->toBe('corporation');
    expect($data['entity']['ticker'])->toBe('TSTC');
    expect($data['entity']['alliance'])->not->toBeNull();
    expect($data['entity']['alliance']['ticker'])->toBe('TSTA');
});

it('can grant alliance access and returns correct JSON shape', function (): void {
    $response = $this->actingAs($this->owner)
        ->postJson("/api/networks/{$this->network->id}/access", [
            'accessible_type' => 'alliance',
            'accessible_id' => $this->alliance->id,
            'accessible_name' => $this->alliance->name,
            'permission' => 'manager',
        ]);

    $response->assertStatus(201);

    $data = $response->json();
    expect($data['accessible_type'])->toBe('alliance');
    expect($data['accessible_id'])->toBe($this->alliance->id);
    expect($data['permission'])->toBe('manager');
    expect($data['entity']['type'])->toBe('alliance');
    expect($data['entity']['ticker'])->toBe('TSTA');
});

it('never returns PHP class names in accessible_type', function (): void {
    $this->actingAs($this->owner)
        ->postJson("/api/networks/{$this->network->id}/access", [
            'accessible_type' => 'corporation',
            'accessible_id' => $this->corporation->id,
            'accessible_name' => $this->corporation->name,
            'permission' => 'viewer',
        ]);

    // Also check the show endpoint
    $showResponse = $this->actingAs($this->owner)
        ->getJson("/api/networks/{$this->network->id}");

    $accesses = $showResponse->json('accesses');
    foreach ($accesses as $access) {
        expect($access['accessible_type'])->toBeIn(['character', 'corporation', 'alliance']);
        expect($access['accessible_type'])->not->toContain('App\\');
    }
});

// ---------------------------------------------------------------------------
// GET /api/networks/{network} — show (access shape within network detail)
// ---------------------------------------------------------------------------

it('returns accesses with entity data in network detail', function (): void {
    $response = $this->actingAs($this->owner)
        ->getJson("/api/networks/{$this->network->id}");

    $response->assertOk()
        ->assertJsonStructure([
            'id',
            'name',
            'slug',
            'accesses' => [
                '*' => [
                    'id',
                    'accessible_type',
                    'accessible_id',
                    'permission',
                    'is_owner',
                    'expires_at',
                    'entity',
                ],
            ],
        ]);

    $ownerAccess = collect($response->json('accesses'))->firstWhere('is_owner', true);
    expect($ownerAccess)->not->toBeNull();
    expect($ownerAccess['accessible_type'])->toBe('character');
    expect($ownerAccess['entity'])->not->toBeNull();
    expect($ownerAccess['entity']['type'])->toBe('character');
});

// ---------------------------------------------------------------------------
// GET /api/networks/{network}/access — index
// ---------------------------------------------------------------------------

it('returns formatted accesses in index', function (): void {
    $response = $this->actingAs($this->owner)
        ->getJson("/api/networks/{$this->network->id}/access");

    $response->assertOk();

    $accesses = $response->json();
    expect($accesses)->toBeArray();
    expect($accesses)->not->toBeEmpty();

    foreach ($accesses as $access) {
        expect($access)->toHaveKeys(['id', 'accessible_type', 'accessible_id', 'permission', 'is_owner', 'expires_at', 'entity']);
        expect($access['accessible_type'])->toBeIn(['character', 'corporation', 'alliance']);
    }
});

// ---------------------------------------------------------------------------
// PUT /api/networks/{network}/access/{access} — update
// ---------------------------------------------------------------------------

it('returns formatted access after update', function (): void {
    $target = User::factory()->create();

    $access = $this->network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $target->character_id,
        'permission' => 'viewer',
    ]);

    $response = $this->actingAs($this->owner)
        ->putJson("/api/networks/{$this->network->id}/access/{$access->id}", [
            'permission' => 'member',
        ]);

    $response->assertOk();

    $data = $response->json();
    expect($data['accessible_type'])->toBe('character');
    expect($data['permission'])->toBe('member');
    expect($data)->toHaveKeys(['id', 'accessible_type', 'accessible_id', 'permission', 'is_owner', 'expires_at', 'entity']);
});

it('prevents modifying owner access', function (): void {
    $ownerAccess = $this->network->accesses()->where('is_owner', true)->first();

    $response = $this->actingAs($this->owner)
        ->putJson("/api/networks/{$this->network->id}/access/{$ownerAccess->id}", [
            'permission' => 'viewer',
        ]);

    $response->assertStatus(403);
});

// ---------------------------------------------------------------------------
// DELETE /api/networks/{network}/access/{access} — destroy
// ---------------------------------------------------------------------------

it('prevents revoking owner access', function (): void {
    $ownerAccess = $this->network->accesses()->where('is_owner', true)->first();

    $response = $this->actingAs($this->owner)
        ->deleteJson("/api/networks/{$this->network->id}/access/{$ownerAccess->id}");

    $response->assertStatus(403);
});

it('can revoke non-owner access', function (): void {
    $target = User::factory()->create();

    $access = $this->network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $target->character_id,
        'permission' => 'viewer',
    ]);

    $response = $this->actingAs($this->owner)
        ->deleteJson("/api/networks/{$this->network->id}/access/{$access->id}");

    $response->assertOk();
    expect(NetworkAccess::find($access->id))->toBeNull();
});

// ---------------------------------------------------------------------------
// Authorization
// ---------------------------------------------------------------------------

it('denies access management to non-managers', function (): void {
    $viewer = User::factory()->create();

    $this->network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $viewer->character_id,
        'permission' => 'viewer',
    ]);

    $response = $this->actingAs($viewer)
        ->postJson("/api/networks/{$this->network->id}/access", [
            'accessible_type' => 'character',
            'accessible_id' => 99999999,
            'accessible_name' => 'Someone',
            'permission' => 'viewer',
        ]);

    $response->assertStatus(403);
});
