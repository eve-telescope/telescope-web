<?php

use App\Models\Alliance;
use App\Models\Corporation;
use App\Models\IntelNetwork;
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

    $this->user = User::factory()->create([
        'corporation_id' => $this->corporation->id,
        'alliance_id' => $this->alliance->id,
    ]);
});

// ---------------------------------------------------------------------------
// GET /api/networks — index
// ---------------------------------------------------------------------------

it('lists networks the user has access to', function (): void {
    $network = IntelNetwork::create(['name' => 'My Net', 'owner_id' => $this->user->id]);
    $network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $this->user->character_id,
        'permission' => 'manager',
        'is_owner' => true,
    ]);

    $response = $this->actingAs($this->user)->getJson('/api/networks');
    $response->assertOk();
    expect($response->json())->toHaveCount(1);
    expect($response->json()[0]['name'])->toBe('My Net');
});

it('excludes networks the user has no access to', function (): void {
    $other = User::factory()->create();
    $network = IntelNetwork::create(['name' => 'Other Net', 'owner_id' => $other->id]);
    $network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $other->character_id,
        'permission' => 'manager',
        'is_owner' => true,
    ]);

    $response = $this->actingAs($this->user)->getJson('/api/networks');
    $response->assertOk();
    expect($response->json())->toHaveCount(0);
});

it('includes networks via corporation access', function (): void {
    $other = User::factory()->create();
    $network = IntelNetwork::create(['name' => 'Corp Net', 'owner_id' => $other->id]);
    $network->accesses()->create([
        'accessible_type' => Corporation::class,
        'accessible_id' => $this->corporation->id,
        'permission' => 'viewer',
    ]);

    $response = $this->actingAs($this->user)->getJson('/api/networks');
    $response->assertOk();
    expect($response->json())->toHaveCount(1);
});

// ---------------------------------------------------------------------------
// POST /api/networks — store
// ---------------------------------------------------------------------------

it('creates a network with owner access', function (): void {
    $response = $this->actingAs($this->user)->postJson('/api/networks', [
        'name' => 'New Network',
    ]);

    $response->assertStatus(201)
        ->assertJsonStructure(['id', 'name', 'slug', 'accesses']);

    expect($response->json('name'))->toBe('New Network');
    expect($response->json('accesses'))->toHaveCount(1);
});

it('rejects empty network name', function (): void {
    $response = $this->actingAs($this->user)->postJson('/api/networks', [
        'name' => '',
    ]);
    $response->assertStatus(422);
});

// ---------------------------------------------------------------------------
// GET /api/networks/{network} — show
// ---------------------------------------------------------------------------

it('shows network detail with accesses and entries', function (): void {
    $network = IntelNetwork::create(['name' => 'Detail Net', 'owner_id' => $this->user->id]);
    $network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $this->user->character_id,
        'permission' => 'manager',
        'is_owner' => true,
    ]);

    $response = $this->actingAs($this->user)->getJson("/api/networks/{$network->id}");

    $response->assertOk()
        ->assertJsonStructure(['id', 'name', 'slug', 'entries', 'accesses']);
});

it('denies show to non-members', function (): void {
    $other = User::factory()->create();
    $network = IntelNetwork::create(['name' => 'Secret Net', 'owner_id' => $other->id]);
    $network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $other->character_id,
        'permission' => 'manager',
        'is_owner' => true,
    ]);

    $response = $this->actingAs($this->user)->getJson("/api/networks/{$network->id}");
    $response->assertStatus(403);
});

// ---------------------------------------------------------------------------
// PUT /api/networks/{network} — update
// ---------------------------------------------------------------------------

it('allows manager to update network name', function (): void {
    $network = IntelNetwork::create(['name' => 'Old Name', 'owner_id' => $this->user->id]);
    $network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $this->user->character_id,
        'permission' => 'manager',
        'is_owner' => true,
    ]);

    $response = $this->actingAs($this->user)->putJson("/api/networks/{$network->id}", [
        'name' => 'New Name',
    ]);

    $response->assertOk();
    expect($response->json('name'))->toBe('New Name');
});

it('denies viewer from updating network', function (): void {
    $owner = User::factory()->create();
    $network = IntelNetwork::create(['name' => 'Net', 'owner_id' => $owner->id]);
    $network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $owner->character_id,
        'permission' => 'manager',
        'is_owner' => true,
    ]);
    $network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $this->user->character_id,
        'permission' => 'viewer',
    ]);

    $response = $this->actingAs($this->user)->putJson("/api/networks/{$network->id}", [
        'name' => 'Hacked',
    ]);
    $response->assertStatus(403);
});

// ---------------------------------------------------------------------------
// DELETE /api/networks/{network} — destroy
// ---------------------------------------------------------------------------

it('allows owner to delete network', function (): void {
    $network = IntelNetwork::create(['name' => 'To Delete', 'owner_id' => $this->user->id]);
    $network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $this->user->character_id,
        'permission' => 'manager',
        'is_owner' => true,
    ]);

    $response = $this->actingAs($this->user)->deleteJson("/api/networks/{$network->id}");
    $response->assertOk();
    expect(IntelNetwork::find($network->id))->toBeNull();
});

it('denies non-owner from deleting network', function (): void {
    $owner = User::factory()->create();
    $network = IntelNetwork::create(['name' => 'Net', 'owner_id' => $owner->id]);
    $network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $owner->character_id,
        'permission' => 'manager',
        'is_owner' => true,
    ]);
    $network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $this->user->character_id,
        'permission' => 'manager',
    ]);

    $response = $this->actingAs($this->user)->deleteJson("/api/networks/{$network->id}");
    $response->assertStatus(403);
});
