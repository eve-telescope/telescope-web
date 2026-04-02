<?php

use App\Models\Alliance;
use App\Models\Corporation;
use App\Models\User;

beforeEach(function (): void {
    $this->alliance = Alliance::create([
        'id' => 99000001,
        'name' => 'Goonswarm Federation',
        'ticker' => 'CONDI',
    ]);

    $this->corporation = Corporation::create([
        'id' => 98000001,
        'name' => 'Karmafleet',
        'ticker' => 'KARMA',
        'alliance_id' => $this->alliance->id,
    ]);

    $this->user = User::factory()->create([
        'character_name' => 'Test Pilot',
        'corporation_id' => $this->corporation->id,
        'alliance_id' => $this->alliance->id,
    ]);
});

// ---------------------------------------------------------------------------
// GET /api/search — with category
// ---------------------------------------------------------------------------

it('searches characters by name', function (): void {
    $response = $this->actingAs($this->user)
        ->getJson('/api/search?query=Test+Pilot&category=character');

    $response->assertOk();

    $results = $response->json();
    expect($results)->not->toBeEmpty();

    $result = $results[0];
    expect($result)->toHaveKeys(['id', 'name', 'category', 'ticker', 'corporation', 'alliance']);
    expect($result['category'])->toBe('character');
    expect($result['name'])->toBe('Test Pilot');
    expect($result['corporation'])->not->toBeNull();
    expect($result['corporation'])->toHaveKeys(['id', 'name', 'ticker']);
});

it('searches corporations by name', function (): void {
    $response = $this->actingAs($this->user)
        ->getJson('/api/search?query=Karma&category=corporation');

    $response->assertOk();

    $results = $response->json();
    expect($results)->not->toBeEmpty();

    $result = $results[0];
    expect($result['category'])->toBe('corporation');
    expect($result['ticker'])->toBe('KARMA');
    expect($result['alliance'])->not->toBeNull();
    expect($result['alliance']['ticker'])->toBe('CONDI');
});

it('searches corporations by ticker', function (): void {
    $response = $this->actingAs($this->user)
        ->getJson('/api/search?query=KARMA&category=corporation');

    $response->assertOk();
    expect($response->json())->not->toBeEmpty();
});

it('searches alliances by name', function (): void {
    $response = $this->actingAs($this->user)
        ->getJson('/api/search?query=Goon&category=alliance');

    $response->assertOk();

    $results = $response->json();
    expect($results)->not->toBeEmpty();

    $result = $results[0];
    expect($result['category'])->toBe('alliance');
    expect($result['ticker'])->toBe('CONDI');
});

// ---------------------------------------------------------------------------
// GET /api/search — without category (searches all)
// ---------------------------------------------------------------------------

it('searches all categories when no category specified', function (): void {
    $response = $this->actingAs($this->user)
        ->getJson('/api/search?query=Test');

    $response->assertOk();

    $results = $response->json();
    expect($results)->not->toBeEmpty();

    // Each result must have the standard shape
    foreach ($results as $result) {
        expect($result)->toHaveKeys(['id', 'name', 'category', 'ticker', 'corporation', 'alliance']);
        expect($result['category'])->toBeIn(['character', 'corporation', 'alliance']);
    }
});

it('returns results from multiple categories', function (): void {
    // Create a corporation whose name overlaps with the user's name
    Corporation::create([
        'id' => 98000002,
        'name' => 'Test Corporation',
        'ticker' => 'TEST',
    ]);

    $response = $this->actingAs($this->user)
        ->getJson('/api/search?query=Test');

    $response->assertOk();

    $categories = collect($response->json())->pluck('category')->unique()->values();
    expect($categories->count())->toBeGreaterThanOrEqual(2);
});

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

it('requires query parameter', function (): void {
    $response = $this->actingAs($this->user)
        ->getJson('/api/search');

    $response->assertStatus(422);
});

it('requires query to be at least 2 characters', function (): void {
    $response = $this->actingAs($this->user)
        ->getJson('/api/search?query=a');

    $response->assertStatus(422);
});

it('rejects invalid category', function (): void {
    $response = $this->actingAs($this->user)
        ->getJson('/api/search?query=test&category=invalid');

    $response->assertStatus(422);
});

// ---------------------------------------------------------------------------
// JSON shape matches Rust/TS expectations
// ---------------------------------------------------------------------------

it('never returns fields that would break Rust deserialization', function (): void {
    $response = $this->actingAs($this->user)
        ->getJson('/api/search?query=Test+Pilot&category=character');

    $results = $response->json();

    foreach ($results as $result) {
        // id must be an integer
        expect($result['id'])->toBeInt();
        // name must be a string
        expect($result['name'])->toBeString();
        // category must be a simple string
        expect($result['category'])->toBeIn(['character', 'corporation', 'alliance']);
        // ticker is nullable
        expect(array_key_exists('ticker', $result))->toBeTrue();
        // corporation/alliance are nullable objects
        expect(array_key_exists('corporation', $result))->toBeTrue();
        expect(array_key_exists('alliance', $result))->toBeTrue();

        if ($result['corporation'] !== null) {
            expect($result['corporation'])->toHaveKeys(['id', 'name', 'ticker']);
            expect($result['corporation']['id'])->toBeInt();
        }
        if ($result['alliance'] !== null) {
            expect($result['alliance'])->toHaveKeys(['id', 'name', 'ticker']);
            expect($result['alliance']['id'])->toBeInt();
        }
    }
});
