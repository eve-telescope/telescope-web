<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Collection;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\HasMany;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $name
 * @property string $ticker
 * @property Carbon|null $last_updated
 * @property Carbon $created_at
 * @property Carbon $updated_at
 * @property-read Collection<int, Corporation> $corporations
 * @property-read Collection<int, User> $users
 */
class Alliance extends Model
{
    public $incrementing = false;

    protected $fillable = [
        'id',
        'name',
        'ticker',
        'last_updated',
    ];

    protected function casts(): array
    {
        return [
            'last_updated' => 'datetime',
        ];
    }

    public function corporations(): HasMany
    {
        return $this->hasMany(Corporation::class);
    }

    public function users(): HasMany
    {
        return $this->hasMany(User::class);
    }
}
