<?php

return [

    /*
    |--------------------------------------------------------------------------
    | Scan Retention Period
    |--------------------------------------------------------------------------
    |
    | The number of days to keep shared scans before they are automatically
    | purged. Set to 0 to keep scans indefinitely.
    |
    */

    'scan_retention_days' => (int) env('SCAN_RETENTION_DAYS', 7),

];
