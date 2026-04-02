<?php

namespace App\Jobs;

use App\Models\Alliance;
use App\Models\Corporation;
use App\Models\User;
use Illuminate\Bus\Queueable;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Foundation\Bus\Dispatchable;
use Illuminate\Queue\InteractsWithQueue;
use Illuminate\Queue\SerializesModels;
use Illuminate\Support\Facades\Http;

class RefreshAffiliations implements ShouldQueue
{
    use Dispatchable, InteractsWithQueue, Queueable, SerializesModels;

    public function handle(): void
    {
        $users = User::all();

        if ($users->isEmpty()) {
            return;
        }

        $characterIds = $users->pluck('character_id')->toArray();

        // ESI supports up to 1000 character IDs per request
        foreach (array_chunk($characterIds, 1000) as $chunk) {
            $response = Http::withHeaders([
                'User-Agent' => 'Telescope | https://eve-telescope.com',
            ])->post('https://esi.evetech.net/latest/characters/affiliation/', $chunk);

            if ($response->failed()) {
                continue;
            }

            foreach ($response->json() as $affiliation) {
                $user = $users->firstWhere('character_id', $affiliation['character_id']);

                if (! $user) {
                    continue;
                }

                $corpId = $affiliation['corporation_id'] ?? null;
                $allianceId = $affiliation['alliance_id'] ?? null;

                if ($allianceId) {
                    $alliance = Alliance::find($allianceId);
                    if (! $alliance || $alliance->last_updated?->diffInHours(now()) > 24) {
                        $this->fetchAndStoreAlliance($allianceId);
                    }
                }

                if ($corpId) {
                    $corp = Corporation::find($corpId);
                    if (! $corp || $corp->last_updated?->diffInHours(now()) > 24) {
                        $this->fetchAndStoreCorporation($corpId);
                    }
                }

                $user->update([
                    'corporation_id' => $corpId,
                    'alliance_id' => $allianceId,
                ]);
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
