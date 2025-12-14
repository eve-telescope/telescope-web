<?php

namespace App\Services;

use Illuminate\Support\Facades\Cache;
use Illuminate\Support\Facades\Http;

class GitHubService
{
    private const REPO = 'eve-telescope/telescope-app';

    private const CACHE_KEY = 'github_latest_version';

    private const CACHE_TTL = 3600; // 1 hour

    public function getLatestVersion(): ?string
    {
        return Cache::remember(self::CACHE_KEY, self::CACHE_TTL, function () {
            try {
                $response = Http::timeout(5)
                    ->get('https://api.github.com/repos/'.self::REPO.'/releases/latest');

                if ($response->successful()) {
                    return $response->json('tag_name');
                }
            } catch (\Exception) {
                // Fall through to return null
            }

            return null;
        });
    }
}
