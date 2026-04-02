<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Foundation\Auth\User as Authenticatable;
use Illuminate\Support\Carbon;
use Laravel\Sanctum\HasApiTokens;

/**
 * @property int $id
 * @property int $character_id
 * @property string $character_name
 * @property string $character_owner_hash
 * @property int|null $corporation_id
 * @property int|null $alliance_id
 * @property Carbon|null $last_active_at
 * @property Carbon $created_at
 * @property Carbon $updated_at
 * @property-read Corporation|null $corporation
 * @property-read Alliance|null $alliance
 */
class User extends Authenticatable
{
    use HasApiTokens, HasFactory;

    protected $fillable = [
        'character_id',
        'character_name',
        'character_owner_hash',
        'corporation_id',
        'alliance_id',
        'last_active_at',
    ];

    protected $hidden = [
        'remember_token',
        'character_owner_hash',
    ];

    protected function casts(): array
    {
        return [
            'character_id' => 'integer',
            'corporation_id' => 'integer',
            'alliance_id' => 'integer',
            'last_active_at' => 'datetime',
        ];
    }

    public function corporation(): BelongsTo
    {
        return $this->belongsTo(Corporation::class);
    }

    public function alliance(): BelongsTo
    {
        return $this->belongsTo(Alliance::class);
    }

    public function getAccessibleIds(): array
    {
        return collect([
            $this->character_id,
            $this->corporation_id,
            $this->alliance_id,
        ])
            ->whereNotNull()
            ->unique()
            ->values()
            ->toArray();
    }

    public function getAuthPassword(): string
    {
        return '';
    }
}
