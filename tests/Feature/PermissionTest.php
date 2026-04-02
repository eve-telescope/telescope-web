<?php

use App\Models\Corporation;
use App\Models\IntelNetwork;
use App\Models\User;

beforeEach(function (): void {
    $this->corporation = Corporation::create(['id' => 98000001, 'name' => 'Test Corp', 'ticker' => 'TSTC']);

    $this->owner = User::factory()->create(['corporation_id' => $this->corporation->id]);
    $this->manager = User::factory()->create();
    $this->member = User::factory()->create();
    $this->viewer = User::factory()->create();
    $this->outsider = User::factory()->create();

    $this->network = IntelNetwork::create(['name' => 'Permission Test', 'owner_id' => $this->owner->id]);
    $this->network->accesses()->create(['accessible_type' => User::class, 'accessible_id' => $this->owner->character_id, 'permission' => 'manager', 'is_owner' => true]);
    $this->network->accesses()->create(['accessible_type' => User::class, 'accessible_id' => $this->manager->character_id, 'permission' => 'manager']);
    $this->network->accesses()->create(['accessible_type' => User::class, 'accessible_id' => $this->member->character_id, 'permission' => 'member']);
    $this->network->accesses()->create(['accessible_type' => User::class, 'accessible_id' => $this->viewer->character_id, 'permission' => 'viewer']);
});

// ---------------------------------------------------------------------------
// Network viewing
// ---------------------------------------------------------------------------

it('viewer can view network detail', function (): void {
    $this->actingAs($this->viewer)->getJson("/api/networks/{$this->network->id}")->assertOk();
});

it('outsider cannot view network detail', function (): void {
    $this->actingAs($this->outsider)->getJson("/api/networks/{$this->network->id}")->assertStatus(403);
});

// ---------------------------------------------------------------------------
// Entry management
// ---------------------------------------------------------------------------

it('member can create entries', function (): void {
    $this->actingAs($this->member)->postJson("/api/networks/{$this->network->id}/entries", [
        'entity_type' => 'character', 'entity_id' => 1, 'entity_name' => 'Test',
    ])->assertStatus(201);
});

it('viewer cannot create entries', function (): void {
    $this->actingAs($this->viewer)->postJson("/api/networks/{$this->network->id}/entries", [
        'entity_type' => 'character', 'entity_id' => 1, 'entity_name' => 'Test',
    ])->assertStatus(403);
});

it('outsider cannot create entries', function (): void {
    $this->actingAs($this->outsider)->postJson("/api/networks/{$this->network->id}/entries", [
        'entity_type' => 'character', 'entity_id' => 1, 'entity_name' => 'Test',
    ])->assertStatus(403);
});

// ---------------------------------------------------------------------------
// Scan sharing
// ---------------------------------------------------------------------------

it('member can share scans', function (): void {
    $this->actingAs($this->member)->postJson("/api/networks/{$this->network->id}/scans", [
        'scan_type' => 'local', 'raw_text' => 'Pilot One',
    ])->assertStatus(201);
});

it('viewer cannot share scans', function (): void {
    $this->actingAs($this->viewer)->postJson("/api/networks/{$this->network->id}/scans", [
        'scan_type' => 'local', 'raw_text' => 'Pilot One',
    ])->assertStatus(403);
});

it('viewer can list scans', function (): void {
    $this->actingAs($this->viewer)->getJson("/api/networks/{$this->network->id}/scans")->assertOk();
});

it('outsider cannot list scans', function (): void {
    $this->actingAs($this->outsider)->getJson("/api/networks/{$this->network->id}/scans")->assertStatus(403);
});

// ---------------------------------------------------------------------------
// Access management
// ---------------------------------------------------------------------------

it('manager can grant access', function (): void {
    $target = User::factory()->create();
    $this->actingAs($this->manager)->postJson("/api/networks/{$this->network->id}/access", [
        'accessible_type' => 'character', 'accessible_id' => $target->character_id, 'accessible_name' => $target->character_name, 'permission' => 'viewer',
    ])->assertStatus(201);
});

it('member cannot grant access', function (): void {
    $target = User::factory()->create();
    $this->actingAs($this->member)->postJson("/api/networks/{$this->network->id}/access", [
        'accessible_type' => 'character', 'accessible_id' => $target->character_id, 'accessible_name' => $target->character_name, 'permission' => 'viewer',
    ])->assertStatus(403);
});

it('viewer cannot grant access', function (): void {
    $target = User::factory()->create();
    $this->actingAs($this->viewer)->postJson("/api/networks/{$this->network->id}/access", [
        'accessible_type' => 'character', 'accessible_id' => $target->character_id, 'accessible_name' => $target->character_name, 'permission' => 'viewer',
    ])->assertStatus(403);
});

// ---------------------------------------------------------------------------
// Network management
// ---------------------------------------------------------------------------

it('manager can update network', function (): void {
    $this->actingAs($this->manager)->putJson("/api/networks/{$this->network->id}", [
        'name' => 'Updated',
    ])->assertOk();
});

it('member cannot update network', function (): void {
    $this->actingAs($this->member)->putJson("/api/networks/{$this->network->id}", [
        'name' => 'Updated',
    ])->assertStatus(403);
});

it('only owner can delete network', function (): void {
    $this->actingAs($this->manager)->deleteJson("/api/networks/{$this->network->id}")->assertStatus(403);
    $this->actingAs($this->owner)->deleteJson("/api/networks/{$this->network->id}")->assertOk();
});

// ---------------------------------------------------------------------------
// Expired access
// ---------------------------------------------------------------------------

it('expired access is denied', function (): void {
    $expired = User::factory()->create();
    $this->network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $expired->character_id,
        'permission' => 'member',
        'expires_at' => now()->subDay(),
    ]);

    $this->actingAs($expired)->getJson("/api/networks/{$this->network->id}")->assertStatus(403);
});
