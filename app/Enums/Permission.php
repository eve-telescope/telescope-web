<?php

namespace App\Enums;

enum Permission: string
{
    case Viewer = 'viewer';
    case Member = 'member';
    case Manager = 'manager';

    public function score(): int
    {
        return match ($this) {
            self::Viewer => 1,
            self::Member => 2,
            self::Manager => 3,
        };
    }

    public function isAtLeast(self $permission): bool
    {
        return $this->score() >= $permission->score();
    }
}
