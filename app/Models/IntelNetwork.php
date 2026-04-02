<?php

namespace App\Models;

use App\Enums\Permission;
use Illuminate\Database\Eloquent\Collection;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Database\Eloquent\Relations\HasMany;
use Illuminate\Support\Carbon;
use Illuminate\Support\Str;

/**
 * @property int $id
 * @property string $name
 * @property string $slug
 * @property int $owner_id
 * @property Carbon $created_at
 * @property Carbon $updated_at
 * @property-read User $owner
 * @property-read Collection<int, NetworkAccess> $accesses
 * @property-read Collection<int, IntelEntry> $entries
 */
class IntelNetwork extends Model
{
    protected $fillable = [
        'name',
        'slug',
        'owner_id',
    ];

    protected static function booted(): void
    {
        static::creating(function (self $network): void {
            if (empty($network->slug)) {
                $network->slug = Str::slug($network->name).'-'.Str::random(6);
            }
        });
    }

    public function owner(): BelongsTo
    {
        return $this->belongsTo(User::class, 'owner_id');
    }

    public function accesses(): HasMany
    {
        return $this->hasMany(NetworkAccess::class, 'intel_network_id');
    }

    public function entries(): HasMany
    {
        return $this->hasMany(IntelEntry::class, 'intel_network_id');
    }

    public function scans(): HasMany
    {
        return $this->hasMany(NetworkScan::class, 'intel_network_id');
    }

    public function getUserPermission(User $user): ?Permission
    {
        $accessibleIds = $user->getAccessibleIds();

        $accesses = $this->accesses()
            ->where(function ($query) use ($accessibleIds): void {
                $query->whereIn('accessible_id', $accessibleIds);
            })
            ->where(function ($query): void {
                $query->whereNull('expires_at')
                    ->orWhere('expires_at', '>', now());
            })
            ->get();

        if ($accesses->isEmpty()) {
            return null;
        }

        return $accesses
            ->map(fn (NetworkAccess $access) => Permission::from($access->permission))
            ->sortByDesc(fn (Permission $p) => $p->score())
            ->first();
    }
}
