<?php

namespace App\Console\Commands;

use App\Models\NetworkAccess;
use Illuminate\Console\Command;

class PurgeExpiredNetworkAccessCommand extends Command
{
    protected $signature = 'network:purge-expired-access';

    protected $description = 'Remove expired network access entries';

    public function handle(): void
    {
        $count = NetworkAccess::whereNotNull('expires_at')
            ->where('expires_at', '<', now())
            ->delete();

        $this->info("Purged {$count} expired network access entries.");
    }
}
