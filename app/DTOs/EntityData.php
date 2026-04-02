<?php

namespace App\DTOs;

use App\Models\Alliance;
use App\Models\Corporation;
use App\Models\User;

class EntityData
{
    public function __construct(
        public readonly int $id,
        public readonly string $name,
        public readonly string $type,
        public readonly ?string $ticker = null,
        public readonly ?AffiliationData $corporation = null,
        public readonly ?AffiliationData $alliance = null,
    ) {}

    public static function fromUser(User $user): self
    {
        return new self(
            id: $user->character_id,
            name: $user->character_name,
            type: 'character',
            corporation: $user->corporation ? AffiliationData::fromModel($user->corporation) : null,
            alliance: $user->alliance ? AffiliationData::fromModel($user->alliance) : null,
        );
    }

    public static function fromCorporation(Corporation $corp): self
    {
        return new self(
            id: $corp->id,
            name: $corp->name,
            type: 'corporation',
            ticker: $corp->ticker,
            alliance: $corp->alliance ? AffiliationData::fromModel($corp->alliance) : null,
        );
    }

    public static function fromAlliance(Alliance $alliance): self
    {
        return new self(
            id: $alliance->id,
            name: $alliance->name,
            type: 'alliance',
            ticker: $alliance->ticker,
        );
    }

    public function toArray(): array
    {
        return [
            'id' => $this->id,
            'name' => $this->name,
            'type' => $this->type,
            'ticker' => $this->ticker,
            'corporation' => $this->corporation?->toArray(),
            'alliance' => $this->alliance?->toArray(),
        ];
    }
}
