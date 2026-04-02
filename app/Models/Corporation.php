<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Collection;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Database\Eloquent\Relations\HasMany;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $name
 * @property string $ticker
 * @property int|null $alliance_id
 * @property int|null $member_count
 * @property Carbon|null $last_updated
 * @property Carbon $created_at
 * @property Carbon $updated_at
 * @property-read Alliance|null $alliance
 * @property-read Collection<int, User> $users
 */
class Corporation extends Model
{
    public $incrementing = false;

    protected $fillable = [
        'id',
        'name',
        'ticker',
        'alliance_id',
        'member_count',
        'last_updated',
    ];

    protected function casts(): array
    {
        return [
            'member_count' => 'integer',
            'last_updated' => 'datetime',
        ];
    }

    public function alliance(): BelongsTo
    {
        return $this->belongsTo(Alliance::class);
    }

    public function users(): HasMany
    {
        return $this->hasMany(User::class);
    }
}
