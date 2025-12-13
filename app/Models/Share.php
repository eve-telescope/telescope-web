<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Str;

class Share extends Model
{
    use HasFactory;

    protected $fillable = [
        'code',
        'pilots',
        'pilot_count',
    ];

    protected function casts(): array
    {
        return [
            'pilots' => 'array',
            'pilot_count' => 'integer',
            'views' => 'integer',
        ];
    }

    public static function generateUniqueCode(): string
    {
        do {
            $code = Str::random(8);
        } while (self::where('code', $code)->exists());

        return $code;
    }

    public function incrementViews(): void
    {
        $this->increment('views');
    }
}
