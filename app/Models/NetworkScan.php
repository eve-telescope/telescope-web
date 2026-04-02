<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int $intel_network_id
 * @property int $user_id
 * @property string $scan_type
 * @property string $raw_text
 * @property string|null $solar_system
 * @property Carbon $created_at
 * @property Carbon $updated_at
 * @property-read IntelNetwork $network
 * @property-read User $user
 */
class NetworkScan extends Model
{
    protected $fillable = [
        'intel_network_id',
        'user_id',
        'scan_type',
        'raw_text',
        'solar_system',
    ];

    public function network(): BelongsTo
    {
        return $this->belongsTo(IntelNetwork::class, 'intel_network_id');
    }

    public function user(): BelongsTo
    {
        return $this->belongsTo(User::class);
    }
}
