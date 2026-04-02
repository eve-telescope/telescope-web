<?php

namespace App\Console\Commands;

use App\Models\NetworkScan;
use Illuminate\Console\Command;

class PurgeOldScansCommand extends Command
{
    protected $signature = 'scans:purge {--days= : Override the retention period in days}';

    protected $description = 'Delete network scans older than the configured retention period';

    public function handle(): void
    {
        $days = $this->option('days') ?? config('telescope-intel.scan_retention_days', 7);

        $count = NetworkScan::where('created_at', '<', now()->subDays((int) $days))->delete();

        $this->info("Purged {$count} scans older than {$days} days.");
    }
}
