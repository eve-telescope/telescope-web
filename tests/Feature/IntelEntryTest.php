<?php

use App\Models\Alliance;
use App\Models\Corporation;
use App\Models\IntelEntry;
use App\Models\IntelNetwork;
use App\Models\User;

beforeEach(function (): void {
    $this->alliance = Alliance::create(['id' => 99000001, 'name' => 'Test Alliance', 'ticker' => 'TSTA']);
    $this->corporation = Corporation::create(['id' => 98000001, 'name' => 'Test Corp', 'ticker' => 'TSTC', 'alliance_id' => $this->alliance->id]);

    $this->owner = User::factory()->create(['corporation_id' => $this->corporation->id, 'alliance_id' => $this->alliance->id]);
    $this->member = User::factory()->create(['corporation_id' => $this->corporation->id]);
    $this->viewer = User::factory()->create();

    $this->network = IntelNetwork::create(['name' => 'Test Network', 'owner_id' => $this->owner->id]);
    $this->network->accesses()->create(['accessible_type' => User::class, 'accessible_id' => $this->owner->character_id, 'permission' => 'manager', 'is_owner' => true]);
    $this->network->accesses()->create(['accessible_type' => User::class, 'accessible_id' => $this->member->character_id, 'permission' => 'member']);
    $this->network->accesses()->create(['accessible_type' => User::class, 'accessible_id' => $this->viewer->character_id, 'permission' => 'viewer']);
});

// ---------------------------------------------------------------------------
// POST /api/networks/{network}/entries — store
// ---------------------------------------------------------------------------

it('allows member to create an entry', function (): void {
    $response = $this->actingAs($this->member)->postJson("/api/networks/{$this->network->id}/entries", [
        'entity_type' => 'character',
        'entity_id' => 12345,
        'entity_name' => 'Bad Guy',
        'color' => '#FF3B3B',
        'label' => 'HOSTILE',
    ]);

    $response->assertStatus(201)
        ->assertJsonStructure(['id', 'entity_type', 'entity_id', 'entity_name', 'color', 'label', 'notes']);
});

it('allows note-only entries without tags', function (): void {
    $response = $this->actingAs($this->member)->postJson("/api/networks/{$this->network->id}/entries", [
        'entity_type' => 'character',
        'entity_id' => 12345,
        'entity_name' => 'Noted Guy',
        'notes' => 'Seen in Jita',
    ]);

    $response->assertStatus(201);
    expect($response->json('notes'))->toBe('Seen in Jita');
    expect($response->json('label'))->toBeNull();
});

it('denies viewer from creating entries', function (): void {
    $response = $this->actingAs($this->viewer)->postJson("/api/networks/{$this->network->id}/entries", [
        'entity_type' => 'character',
        'entity_id' => 12345,
        'entity_name' => 'Test',
        'label' => 'HOSTILE',
    ]);

    $response->assertStatus(403);
});

it('upserts entry for same entity', function (): void {
    $this->actingAs($this->member)->postJson("/api/networks/{$this->network->id}/entries", [
        'entity_type' => 'character',
        'entity_id' => 12345,
        'entity_name' => 'Target',
        'label' => 'HOSTILE',
    ]);

    $this->actingAs($this->member)->postJson("/api/networks/{$this->network->id}/entries", [
        'entity_type' => 'character',
        'entity_id' => 12345,
        'entity_name' => 'Target',
        'label' => 'HOSTILE | SPY',
    ]);

    expect($this->network->entries()->where('entity_id', 12345)->count())->toBe(1);
    expect($this->network->entries()->where('entity_id', 12345)->first()->label)->toBe('HOSTILE | SPY');
});

// ---------------------------------------------------------------------------
// PUT /api/networks/{network}/entries/{entry} — update
// ---------------------------------------------------------------------------

it('allows member to update an entry', function (): void {
    $entry = $this->network->entries()->create([
        'entity_type' => 'character',
        'entity_id' => 12345,
        'entity_name' => 'Target',
        'label' => 'HOSTILE',
        'added_by' => $this->owner->id,
    ]);

    $response = $this->actingAs($this->member)->putJson("/api/networks/{$this->network->id}/entries/{$entry->id}", [
        'label' => 'FRIENDLY',
        'notes' => 'Actually a friend',
    ]);

    $response->assertOk();
    expect($response->json('label'))->toBe('FRIENDLY');
    expect($response->json('notes'))->toBe('Actually a friend');
});

// ---------------------------------------------------------------------------
// DELETE /api/networks/{network}/entries/{entry} — destroy
// ---------------------------------------------------------------------------

it('allows member to delete an entry', function (): void {
    $entry = $this->network->entries()->create([
        'entity_type' => 'character',
        'entity_id' => 12345,
        'entity_name' => 'Target',
        'label' => 'HOSTILE',
        'added_by' => $this->owner->id,
    ]);

    $response = $this->actingAs($this->member)->deleteJson("/api/networks/{$this->network->id}/entries/{$entry->id}");
    $response->assertOk();
    expect(IntelEntry::find($entry->id))->toBeNull();
});

it('denies viewer from deleting entries', function (): void {
    $entry = $this->network->entries()->create([
        'entity_type' => 'character',
        'entity_id' => 12345,
        'entity_name' => 'Target',
        'label' => 'HOSTILE',
        'added_by' => $this->owner->id,
    ]);

    $response = $this->actingAs($this->viewer)->deleteJson("/api/networks/{$this->network->id}/entries/{$entry->id}");
    $response->assertStatus(403);
});

// ---------------------------------------------------------------------------
// GET /api/networks/{network}/entries — index
// ---------------------------------------------------------------------------

it('allows viewer to list entries', function (): void {
    $this->network->entries()->create([
        'entity_type' => 'character',
        'entity_id' => 12345,
        'entity_name' => 'Target',
        'label' => 'HOSTILE',
        'added_by' => $this->owner->id,
    ]);

    $response = $this->actingAs($this->viewer)->getJson("/api/networks/{$this->network->id}/entries");
    $response->assertOk();
    expect($response->json())->toHaveCount(1);
});

// ---------------------------------------------------------------------------
// GET /api/intel/lookup — bulk lookup
// ---------------------------------------------------------------------------

it('returns entries matching entity IDs across accessible networks', function (): void {
    $this->network->entries()->create([
        'entity_type' => 'character',
        'entity_id' => 12345,
        'entity_name' => 'Target',
        'label' => 'HOSTILE',
        'added_by' => $this->owner->id,
    ]);

    $response = $this->actingAs($this->viewer)->getJson('/api/intel/lookup?entity_ids[]=12345');

    $response->assertOk();
    expect($response->json())->toHaveCount(1);
    expect($response->json()[0]['entity_id'])->toBe(12345);
    expect($response->json()[0])->toHaveKeys(['id', 'intel_network_id', 'network_name', 'entity_type', 'entity_id', 'entity_name', 'color', 'label', 'notes']);
});

it('excludes entries from inaccessible networks', function (): void {
    $other = User::factory()->create();
    $otherNetwork = IntelNetwork::create(['name' => 'Other', 'owner_id' => $other->id]);
    $otherNetwork->accesses()->create(['accessible_type' => User::class, 'accessible_id' => $other->character_id, 'permission' => 'manager', 'is_owner' => true]);
    $otherNetwork->entries()->create([
        'entity_type' => 'character',
        'entity_id' => 99999,
        'entity_name' => 'Secret',
        'label' => 'SPY',
        'added_by' => $other->id,
    ]);

    $response = $this->actingAs($this->viewer)->getJson('/api/intel/lookup?entity_ids[]=99999');
    $response->assertOk();
    expect($response->json())->toHaveCount(0);
});
