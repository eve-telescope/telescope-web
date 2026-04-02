<?php

namespace App\Services;

use App\Models\Alliance;
use App\Models\Corporation;
use App\Models\User;
use Illuminate\Support\Facades\Http;
use Laravel\Socialite\Facades\Socialite;

class EveAuthService
{
    public function getUser(): User
    {
        $socialiteUser = Socialite::driver('eveonline')->user();

        $characterId = (int) $socialiteUser->attributes['character_id'];
        $characterName = $socialiteUser->name
            ?? $socialiteUser->attributes['character_name']
            ?? $socialiteUser->user['CharacterName']
            ?? 'Unknown';
        $ownerHash = $socialiteUser->attributes['character_owner_hash']
            ?? $socialiteUser->user['CharacterOwnerHash']
            ?? '';

        $affiliations = $this->fetchAffiliations($characterId);

        if ($affiliations) {
            $this->ensureEntitiesExist($affiliations);
        }

        $user = User::updateOrCreate(
            ['character_id' => $characterId],
            [
                'character_name' => $characterName,
                'character_owner_hash' => $ownerHash,
                'corporation_id' => $affiliations['corporation_id'] ?? null,
                'alliance_id' => $affiliations['alliance_id'] ?? null,
            ],
        );

        return $user;
    }

    private function fetchAffiliations(int $characterId): ?array
    {
        $response = Http::withHeaders([
            'User-Agent' => 'Telescope | https://eve-telescope.com',
        ])->post('https://esi.evetech.net/latest/characters/affiliation/', [$characterId]);

        if ($response->failed() || empty($response->json())) {
            return null;
        }

        $data = $response->json()[0];

        return [
            'corporation_id' => $data['corporation_id'] ?? null,
            'alliance_id' => $data['alliance_id'] ?? null,
        ];
    }

    private function ensureEntitiesExist(array $affiliations): void
    {
        if ($allianceId = $affiliations['alliance_id'] ?? null) {
            $alliance = Alliance::find($allianceId);

            if (! $alliance || $alliance->last_updated?->diffInHours(now()) > 24) {
                $this->fetchAndStoreAlliance($allianceId);
            }
        }

        if ($corpId = $affiliations['corporation_id'] ?? null) {
            $corp = Corporation::find($corpId);

            if (! $corp || $corp->last_updated?->diffInHours(now()) > 24) {
                $this->fetchAndStoreCorporation($corpId);
            }
        }
    }

    private function fetchAndStoreAlliance(int $allianceId): void
    {
        $response = Http::withHeaders([
            'User-Agent' => 'Telescope | https://eve-telescope.com',
        ])->get("https://esi.evetech.net/latest/alliances/{$allianceId}/");

        if ($response->failed()) {
            return;
        }

        $data = $response->json();

        Alliance::updateOrCreate(
            ['id' => $allianceId],
            [
                'name' => $data['name'],
                'ticker' => $data['ticker'],
                'last_updated' => now(),
            ],
        );
    }

    private function fetchAndStoreCorporation(int $corpId): void
    {
        $response = Http::withHeaders([
            'User-Agent' => 'Telescope | https://eve-telescope.com',
        ])->get("https://esi.evetech.net/latest/corporations/{$corpId}/");

        if ($response->failed()) {
            return;
        }

        $data = $response->json();

        Corporation::updateOrCreate(
            ['id' => $corpId],
            [
                'name' => $data['name'],
                'ticker' => $data['ticker'],
                'alliance_id' => $data['alliance_id'] ?? null,
                'member_count' => $data['member_count'] ?? null,
                'last_updated' => now(),
            ],
        );
    }
}
