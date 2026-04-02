<?php

namespace Database\Factories;

use App\Models\User;
use Illuminate\Database\Eloquent\Factories\Factory;
use Illuminate\Support\Str;

/**
 * @extends Factory<User>
 */
class UserFactory extends Factory
{
    public function definition(): array
    {
        return [
            'character_id' => fake()->unique()->numberBetween(90000000, 99999999),
            'character_name' => fake()->userName(),
            'character_owner_hash' => Str::random(40),
        ];
    }
}
