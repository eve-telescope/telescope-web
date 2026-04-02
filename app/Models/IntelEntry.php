<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int $intel_network_id
 * @property string $entity_type
 * @property int $entity_id
 * @property string $entity_name
 * @property string $color
 * @property string $label
 * @property string|null $notes
 * @property int $added_by
 * @property Carbon $created_at
 * @property Carbon $updated_at
 * @property-read IntelNetwork $network
 * @property-read User $addedBy
 */
class IntelEntry extends Model
{
    protected $fillable = [
        'intel_network_id',
        'entity_type',
        'entity_id',
        'entity_name',
        'color',
        'label',
        'notes',
        'added_by',
    ];

    public function network(): BelongsTo
    {
        return $this->belongsTo(IntelNetwork::class, 'intel_network_id');
    }

    public function addedBy(): BelongsTo
    {
        return $this->belongsTo(User::class, 'added_by');
    }
}
