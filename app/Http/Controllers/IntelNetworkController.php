<?php

namespace App\Http\Controllers;

use App\DTOs\NetworkAccessData;
use App\Enums\Permission;
use App\Models\IntelNetwork;
use App\Models\NetworkAccess;
use App\Models\User;
use Illuminate\Http\Request;
use Illuminate\Support\Str;

class IntelNetworkController extends Controller
{
    public function index(Request $request)
    {
        /** @var User $user */
        $user = $request->user();
        $accessibleIds = $user->getAccessibleIds();

        $networks = IntelNetwork::whereHas('accesses', function ($query) use ($accessibleIds): void {
            $query->whereIn('accessible_id', $accessibleIds)->notExpired();
        })
            ->withCount('entries')
            ->get();

        return response()->json($networks);
    }

    public function store(Request $request)
    {
        $request->validate([
            'name' => ['required', 'string', 'max:255'],
        ]);

        /** @var User $user */
        $user = $request->user();

        $network = IntelNetwork::create([
            'name' => $request->name,
            'slug' => Str::slug($request->name).'-'.Str::random(6),
            'owner_id' => $user->id,
        ]);

        // Grant owner access
        $network->accesses()->create([
            'accessible_type' => User::class,
            'accessible_id' => $user->character_id,
            'permission' => Permission::Manager->value,
            'is_owner' => true,
        ]);

        return response()->json($network->load('accesses'), 201);
    }

    public function show(Request $request, IntelNetwork $network)
    {
        $this->authorize('view', $network);

        $network->load(['entries.addedBy', 'accesses']);
        $network->loadCount('entries');

        $accesses = $network->accesses->map(
            fn (NetworkAccess $access) => NetworkAccessData::fromModel($access)->toArray()
        );

        $data = $network->toArray();
        $data['accesses'] = $accesses->values();

        return response()->json($data);
    }

    public function update(Request $request, IntelNetwork $network)
    {
        $this->authorize('update', $network);

        $request->validate([
            'name' => ['required', 'string', 'max:255'],
        ]);

        $network->update([
            'name' => $request->name,
        ]);

        return response()->json($network);
    }

    public function destroy(Request $request, IntelNetwork $network)
    {
        $this->authorize('delete', $network);

        $network->delete();

        return response()->json(['message' => 'Network deleted']);
    }
}
