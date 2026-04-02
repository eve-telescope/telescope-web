<?php

namespace App\DTOs;

use App\Models\Alliance;
use App\Models\Corporation;
use App\Models\NetworkAccess;
use App\Models\User;

class NetworkAccessData
{
    public const MORPH_TO_TYPE = [
        User::class => 'character',
        Corporation::class => 'corporation',
        Alliance::class => 'alliance',
    ];

    public const TYPE_TO_MORPH = [
        'character' => User::class,
        'corporation' => Corporation::class,
        'alliance' => Alliance::class,
    ];

    public function __construct(
        public readonly int $id,
        public readonly string $accessible_type,
        public readonly int $accessible_id,
        public readonly string $permission,
        public readonly bool $is_owner,
        public readonly ?string $expires_at,
        public readonly ?EntityData $entity,
    ) {}

    public static function fromModel(NetworkAccess $access): self
    {
        return new self(
            id: $access->id,
            accessible_type: self::MORPH_TO_TYPE[$access->accessible_type] ?? $access->accessible_type,
            accessible_id: $access->accessible_id,
            permission: $access->permission,
            is_owner: (bool) $access->is_owner,
            expires_at: $access->expires_at?->toIso8601String(),
            entity: self::resolveEntity($access),
        );
    }

    private static function resolveEntity(NetworkAccess $access): ?EntityData
    {
        return match ($access->accessible_type) {
            User::class => ($user = User::where('character_id', $access->accessible_id)->first())
                ? EntityData::fromUser($user)
                : null,
            Corporation::class => ($corp = Corporation::find($access->accessible_id))
                ? EntityData::fromCorporation($corp)
                : null,
            Alliance::class => ($alliance = Alliance::find($access->accessible_id))
                ? EntityData::fromAlliance($alliance)
                : null,
            default => null,
        };
    }

    /**
     * Convert a simple type string ('character', 'corporation', 'alliance')
     * to the corresponding Eloquent morph class name.
     */
    public static function morphClass(string $type): string
    {
        return self::TYPE_TO_MORPH[$type]
            ?? throw new \InvalidArgumentException("Unknown accessible type: {$type}");
    }

    public function toArray(): array
    {
        return [
            'id' => $this->id,
            'accessible_type' => $this->accessible_type,
            'accessible_id' => $this->accessible_id,
            'permission' => $this->permission,
            'is_owner' => $this->is_owner,
            'expires_at' => $this->expires_at,
            'entity' => $this->entity?->toArray(),
        ];
    }
}
