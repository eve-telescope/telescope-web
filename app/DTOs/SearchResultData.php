<?php

namespace App\DTOs;

class SearchResultData
{
    public function __construct(
        public readonly int $id,
        public readonly string $name,
        public readonly string $category,
        public readonly ?string $ticker = null,
        public readonly ?AffiliationData $corporation = null,
        public readonly ?AffiliationData $alliance = null,
    ) {}

    public function toArray(): array
    {
        return [
            'id' => $this->id,
            'name' => $this->name,
            'category' => $this->category,
            'ticker' => $this->ticker,
            'corporation' => $this->corporation?->toArray(),
            'alliance' => $this->alliance?->toArray(),
        ];
    }
}
