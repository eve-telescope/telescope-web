<?php

use App\Models\IntelNetwork;
use App\Models\NetworkScan;
use App\Models\User;

beforeEach(function (): void {
    $this->owner = User::factory()->create();
    $this->network = IntelNetwork::create(['name' => 'Test', 'owner_id' => $this->owner->id]);
});

function createScanAt($network, $owner, string $text, int $daysAgo): NetworkScan
{
    $scan = $network->scans()->create([
        'user_id' => $owner->id,
        'scan_type' => 'local',
        'raw_text' => $text,
    ]);

    NetworkScan::where('id', $scan->id)->update([
        'created_at' => now()->subDays($daysAgo),
    ]);

    return $scan->fresh();
}

it('deletes scans older than configured retention', function (): void {
    createScanAt($this->network, $this->owner, 'Old scan', 8);
    createScanAt($this->network, $this->owner, 'Recent scan', 3);

    $this->artisan('scans:purge')->assertExitCode(0);

    expect(NetworkScan::count())->toBe(1);
    expect(NetworkScan::first()->raw_text)->toBe('Recent scan');
});

it('respects --days override', function (): void {
    createScanAt($this->network, $this->owner, 'Two days old', 2);

    $this->artisan('scans:purge --days=1')->assertExitCode(0);

    expect(NetworkScan::count())->toBe(0);
});

it('keeps all scans when none are expired', function (): void {
    $this->network->scans()->create([
        'user_id' => $this->owner->id,
        'scan_type' => 'local',
        'raw_text' => 'Fresh scan',
    ]);

    $this->artisan('scans:purge')->assertExitCode(0);

    expect(NetworkScan::count())->toBe(1);
});

it('respects config value', function (): void {
    config(['telescope-intel.scan_retention_days' => 3]);

    createScanAt($this->network, $this->owner, 'Four days old', 4);
    createScanAt($this->network, $this->owner, 'Two days old', 2);

    $this->artisan('scans:purge')->assertExitCode(0);

    expect(NetworkScan::count())->toBe(1);
    expect(NetworkScan::first()->raw_text)->toBe('Two days old');
});
