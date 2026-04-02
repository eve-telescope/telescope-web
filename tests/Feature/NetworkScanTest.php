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
// POST /api/networks/{network}/scans
// ---------------------------------------------------------------------------

it('allows member to share a local scan', function (): void {
    $response = $this->actingAs($this->owner)
        ->postJson("/api/networks/{$this->network->id}/scans", [
            'scan_type' => 'local',
            'raw_text' => "Pilot One\nPilot Two\nPilot Three",
        ]);

    $response->assertStatus(201)
        ->assertJsonStructure([
            'id',
            'scan_type',
            'raw_text',
            'solar_system',
            'created_at',
            'submitted_by' => ['id', 'character_name'],
        ]);

    $data = $response->json();
    expect($data['scan_type'])->toBe('local');
    expect($data['raw_text'])->toBe("Pilot One\nPilot Two\nPilot Three");
    expect($data['solar_system'])->toBeNull();
    expect($data['submitted_by']['id'])->toBe($this->owner->character_id);
    expect($data['submitted_by']['character_name'])->toBe($this->owner->character_name);
});

it('allows member to share a dscan', function (): void {
    $response = $this->actingAs($this->owner)
        ->postJson("/api/networks/{$this->network->id}/scans", [
            'scan_type' => 'dscan',
            'raw_text' => "12345\tShip Name\tRifter\t100 km",
            'solar_system' => 'Jita',
        ]);

    $response->assertStatus(201);

    $data = $response->json();
    expect($data['scan_type'])->toBe('dscan');
    expect($data['solar_system'])->toBe('Jita');
});

it('denies viewer from sharing scans', function (): void {
    $viewer = User::factory()->create();
    $this->network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $viewer->character_id,
        'permission' => 'viewer',
    ]);

    $response = $this->actingAs($viewer)
        ->postJson("/api/networks/{$this->network->id}/scans", [
            'scan_type' => 'local',
            'raw_text' => 'Pilot One',
        ]);

    $response->assertStatus(403);
});

it('rejects invalid scan type', function (): void {
    $response = $this->actingAs($this->owner)
        ->postJson("/api/networks/{$this->network->id}/scans", [
            'scan_type' => 'invalid',
            'raw_text' => 'test',
        ]);

    $response->assertStatus(422);
});

it('rejects missing raw text', function (): void {
    $response = $this->actingAs($this->owner)
        ->postJson("/api/networks/{$this->network->id}/scans", [
            'scan_type' => 'local',
        ]);

    $response->assertStatus(422);
});

// ---------------------------------------------------------------------------
// GET /api/networks/{network}/scans
// ---------------------------------------------------------------------------

it('allows viewer to list scan history', function (): void {
    $viewer = User::factory()->create();
    $this->network->accesses()->create([
        'accessible_type' => User::class,
        'accessible_id' => $viewer->character_id,
        'permission' => 'viewer',
    ]);

    // Create some scans
    $this->network->scans()->create([
        'user_id' => $this->owner->id,
        'scan_type' => 'local',
        'raw_text' => 'Pilot A',
    ]);
    $this->network->scans()->create([
        'user_id' => $this->owner->id,
        'scan_type' => 'dscan',
        'raw_text' => "12345\tName\tType\t100km",
    ]);

    $response = $this->actingAs($viewer)
        ->getJson("/api/networks/{$this->network->id}/scans");

    $response->assertOk()
        ->assertJsonStructure([
            'data' => [
                '*' => [
                    'id',
                    'scan_type',
                    'raw_text',
                    'solar_system',
                    'created_at',
                    'submitted_by',
                ],
            ],
            'current_page',
            'last_page',
        ]);

    expect($response->json('data'))->toHaveCount(2);
});

it('returns scans in newest-first order', function (): void {
    $this->network->scans()->create([
        'user_id' => $this->owner->id,
        'scan_type' => 'local',
        'raw_text' => 'First scan',
    ]);
    $this->network->scans()->create([
        'user_id' => $this->owner->id,
        'scan_type' => 'local',
        'raw_text' => 'Second scan',
    ]);

    $response = $this->actingAs($this->owner)
        ->getJson("/api/networks/{$this->network->id}/scans");

    $scans = $response->json('data');
    expect($scans[0]['raw_text'])->toBe('Second scan');
    expect($scans[1]['raw_text'])->toBe('First scan');
});

it('paginates scan history', function (): void {
    for ($i = 0; $i < 25; $i++) {
        $this->network->scans()->create([
            'user_id' => $this->owner->id,
            'scan_type' => 'local',
            'raw_text' => "Scan {$i}",
        ]);
    }

    $response = $this->actingAs($this->owner)
        ->getJson("/api/networks/{$this->network->id}/scans");

    expect($response->json('data'))->toHaveCount(20);
    expect($response->json('last_page'))->toBe(2);

    $page2 = $this->actingAs($this->owner)
        ->getJson("/api/networks/{$this->network->id}/scans?page=2");

    expect($page2->json('data'))->toHaveCount(5);
});
