<?php

namespace App\Http\Controllers;

use App\DTOs\NetworkAccessData;
use App\Models\IntelNetwork;
use App\Models\NetworkAccess;
use Illuminate\Http\Request;

class NetworkAccessController extends Controller
{
    public function index(Request $request, IntelNetwork $network)
    {
        $this->authorize('manageAccess', $network);

        $accesses = $network->accesses->map(
            fn (NetworkAccess $access) => NetworkAccessData::fromModel($access)->toArray()
        );

        return response()->json($accesses->values());
    }

    public function store(Request $request, IntelNetwork $network)
    {
        $this->authorize('manageAccess', $network);

        $request->validate([
            'accessible_type' => ['required', 'string', 'in:character,corporation,alliance'],
            'accessible_id' => ['required', 'integer'],
            'accessible_name' => ['required', 'string', 'max:255'],
            'permission' => ['required', 'string', 'in:viewer,member,manager'],
            'expires_at' => ['nullable', 'date', 'after:now'],
        ]);

        $access = $network->accesses()->updateOrCreate(
            [
                'accessible_type' => NetworkAccessData::morphClass($request->accessible_type),
                'accessible_id' => $request->accessible_id,
            ],
            [
                'permission' => $request->permission,
                'expires_at' => $request->expires_at,
            ],
        );

        return response()->json(NetworkAccessData::fromModel($access)->toArray(), 201);
    }

    public function update(Request $request, IntelNetwork $network, NetworkAccess $access)
    {
        $this->authorize('manageAccess', $network);

        if ($access->is_owner) {
            return response()->json(['message' => 'Cannot modify owner access'], 403);
        }

        $request->validate([
            'permission' => ['sometimes', 'string', 'in:viewer,member,manager'],
            'expires_at' => ['nullable', 'date', 'after:now'],
        ]);

        $access->update($request->only(['permission', 'expires_at']));

        return response()->json(NetworkAccessData::fromModel($access)->toArray());
    }

    public function destroy(Request $request, IntelNetwork $network, NetworkAccess $access)
    {
        $this->authorize('manageAccess', $network);

        if ($access->is_owner) {
            return response()->json(['message' => 'Cannot revoke owner access'], 403);
        }

        $access->delete();

        return response()->json(['message' => 'Access revoked']);
    }
}
