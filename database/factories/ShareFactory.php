<?php

namespace Database\Factories;

use App\Models\Share;
use Illuminate\Database\Eloquent\Factories\Factory;
use Illuminate\Support\Str;

/**
 * @extends Factory<Share>
 */
class ShareFactory extends Factory
{
    public function definition(): array
    {
        $pilots = $this->faker->words(rand(5, 20));

        return [
            'code' => Str::random(8),
            'pilots' => $pilots,
            'pilot_count' => count($pilots),
        ];
    }
}
