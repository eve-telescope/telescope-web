<?php

namespace App\Http\Controllers;

use App\DTOs\AffiliationData;
use App\DTOs\SearchResultData;
use App\Models\Alliance;
use App\Models\Corporation;
use App\Models\User;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Http;

class EntitySearchController extends Controller
{
    public function search(Request $request)
    {
        $request->validate([
            'query' => ['required', 'string', 'min:2', 'max:255'],
            'category' => ['nullable', 'string', 'in:character,corporation,alliance'],
        ]);

        $query = $request->query('query');
        $category = $request->query('category');

        $categories = $category ? [$category] : ['character', 'corporation', 'alliance'];

        $results = collect();
        foreach ($categories as $cat) {
            $local = $this->searchLocal($query, $cat);
            if ($local->isNotEmpty()) {
                $results = $results->concat($local);
            } else {
                $results = $results->concat($this->searchEsi($query, $cat));
            }
        }

        return response()->json(
            $results->map(fn (SearchResultData $dto) => $dto->toArray())->values()
        );
    }

    private function searchLocal(string $query, string $category)
    {
        return match ($category) {
            'character' => User::where('character_name', 'like', "%{$query}%")
                ->limit(15)
                ->get()
                ->map(fn (User $user) => new SearchResultData(
                    id: $user->character_id,
                    name: $user->character_name,
                    category: 'character',
                    corporation: $user->corporation ? AffiliationData::fromModel($user->corporation) : null,
                    alliance: $user->alliance ? AffiliationData::fromModel($user->alliance) : null,
                )),
            'corporation' => Corporation::where('name', 'like', "%{$query}%")
                ->orWhere('ticker', 'like', "%{$query}%")
                ->limit(15)
                ->get()
                ->map(fn (Corporation $corp) => new SearchResultData(
                    id: $corp->id,
                    name: $corp->name,
                    category: 'corporation',
                    ticker: $corp->ticker,
                    alliance: $corp->alliance ? AffiliationData::fromModel($corp->alliance) : null,
                )),
            'alliance' => Alliance::where('name', 'like', "%{$query}%")
                ->orWhere('ticker', 'like', "%{$query}%")
                ->limit(15)
                ->get()
                ->map(fn (Alliance $alliance) => new SearchResultData(
                    id: $alliance->id,
                    name: $alliance->name,
                    category: 'alliance',
                    ticker: $alliance->ticker,
                )),
        };
    }

    private function searchEsi(string $query, string $category)
    {
        $response = Http::withHeaders([
            'User-Agent' => 'Telescope | https://eve-telescope.com',
        ])->get('https://esi.evetech.net/latest/search/', [
            'categories' => $category,
            'search' => $query,
            'strict' => 'false',
        ]);

        if ($response->failed()) {
            return collect();
        }

        $ids = array_slice($response->json($category, []), 0, 10);

        if (empty($ids)) {
            return collect();
        }

        $namesResponse = Http::withHeaders([
            'User-Agent' => 'Telescope | https://eve-telescope.com',
        ])->post('https://esi.evetech.net/latest/universe/names/', $ids);

        if ($namesResponse->failed()) {
            return collect();
        }

        return collect($namesResponse->json())->map(fn (array $item) => new SearchResultData(
            id: $item['id'],
            name: $item['name'],
            category: $category,
        ));
    }
}
