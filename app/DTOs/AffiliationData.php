<?php

namespace App\DTOs;

use App\Models\Alliance;
use App\Models\Corporation;

class AffiliationData
{
    public function __construct(
        public readonly int $id,
        public readonly string $name,
        public readonly string $ticker,
    ) {}

    public static function fromModel(Corporation|Alliance $model): self
    {
        return new self(
            id: $model->id,
            name: $model->name,
            ticker: $model->ticker,
        );
    }

    public function toArray(): array
    {
        return [
            'id' => $this->id,
            'name' => $this->name,
            'ticker' => $this->ticker,
        ];
    }
}
