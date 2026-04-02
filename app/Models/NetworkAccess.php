<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Database\Eloquent\Relations\MorphTo;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int $intel_network_id
 * @property string $accessible_type
 * @property int $accessible_id
 * @property string $permission
 * @property bool $is_owner
 * @property Carbon|null $expires_at
 * @property Carbon $created_at
 * @property Carbon $updated_at
 * @property-read IntelNetwork $network
 * @property-read Alliance|Corporation|User $accessible
 */
class NetworkAccess extends Model
{
    protected $table = 'network_access';

    protected $fillable = [
        'intel_network_id',
        'accessible_type',
        'accessible_id',
        'permission',
        'is_owner',
        'expires_at',
    ];

    protected function casts(): array
    {
        return [
            'is_owner' => 'boolean',
            'expires_at' => 'datetime',
        ];
    }

    public function network(): BelongsTo
    {
        return $this->belongsTo(IntelNetwork::class, 'intel_network_id');
    }

    public function accessible(): MorphTo
    {
        return $this->morphTo();
    }

    public function isExpired(): bool
    {
        return $this->expires_at !== null && $this->expires_at->isPast();
    }

    public function scopeNotExpired($query)
    {
        return $query->where(function ($q): void {
            $q->whereNull('expires_at')
                ->orWhere('expires_at', '>', now());
        });
    }
}
