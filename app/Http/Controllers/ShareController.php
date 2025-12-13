<?php

namespace App\Http\Controllers;

use App\Models\Share;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Inertia\Inertia;
use Inertia\Response;

class ShareController extends Controller
{
    public function store(Request $request): JsonResponse
    {
        $validated = $request->validate([
            'pilots' => ['required', 'array', 'min:1', 'max:500'],
            'pilots.*' => ['required', 'string', 'max:100'],
        ]);

        $pilots = array_values(array_unique(array_filter(
            array_map('trim', $validated['pilots'])
        )));

        $share = Share::create([
            'code' => Share::generateUniqueCode(),
            'pilots' => $pilots,
            'pilot_count' => count($pilots),
        ]);

        return response()->json([
            'code' => $share->code,
            'url' => route('share.show', $share->code, absolute: true),
        ]);
    }

    public function fetch(string $code): JsonResponse
    {
        $share = Share::where('code', $code)->firstOrFail();

        return response()->json([
            'code' => $share->code,
            'pilots' => $share->pilots,
            'pilotCount' => $share->pilot_count,
        ]);
    }

    public function show(string $code): Response
    {
        $share = Share::where('code', $code)->firstOrFail();
        $share->incrementViews();

        return Inertia::render('Share', [
            'share' => [
                'code' => $share->code,
                'pilots' => $share->pilots,
                'pilotCount' => $share->pilot_count,
                'views' => $share->views,
                'createdAt' => $share->created_at->toISOString(),
            ],
        ]);
    }
}
