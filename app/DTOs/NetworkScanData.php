<?php

namespace App\DTOs;

use App\Models\NetworkScan;

class NetworkScanData
{
    public function __construct(
        public readonly int $id,
        public readonly string $scan_type,
        public readonly string $raw_text,
        public readonly ?string $solar_system,
        public readonly string $created_at,
        public readonly ?array $submitted_by,
    ) {}

    public static function fromModel(NetworkScan $scan): self
    {
        return new self(
            id: $scan->id,
            scan_type: $scan->scan_type,
            raw_text: $scan->raw_text,
            solar_system: $scan->solar_system,
            created_at: $scan->created_at->toIso8601String(),
            submitted_by: $scan->user ? [
                'id' => $scan->user->character_id,
                'character_name' => $scan->user->character_name,
            ] : null,
        );
    }

    public function toArray(): array
    {
        return [
            'id' => $this->id,
            'scan_type' => $this->scan_type,
            'raw_text' => $this->raw_text,
            'solar_system' => $this->solar_system,
            'created_at' => $this->created_at,
            'submitted_by' => $this->submitted_by,
        ];
    }
}
