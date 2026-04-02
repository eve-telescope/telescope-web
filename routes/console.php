<?php

use App\Jobs\RefreshAffiliations;
use Illuminate\Support\Facades\Schedule;

Schedule::job(new RefreshAffiliations)->hourly();
Schedule::command('network:purge-expired-access')->hourly();
