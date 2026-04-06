<?php

namespace App\Http\Controllers;

use App\Events\IntelEntryCreated;
use App\Events\IntelEntryDeleted;
use App\Events\IntelEntryUpdated;
use App\Models\IntelEntry;
use App\Models\IntelNetwork;
use App\Models\User;
use Illuminate\Http\Request;

class IntelEntryController extends Controller
{
    public function index(Request $request, IntelNetwork $network)
    {
        $this->authorize('view', $network);

        return response()->json($network->entries()->with('addedBy')->get());
    }

    public function store(Request $request, IntelNetwork $network)
    {
        $this->authorize('addEntry', $network);

        $request->validate([
            'entity_type' => ['required', 'string', 'in:character,corporation,alliance'],
            'entity_id' => ['required', 'integer'],
            'entity_name' => ['required', 'string', 'max:255'],
            'color' => ['nullable', 'string', 'regex:/^#[0-9A-Fa-f]{6}$/'],
            'label' => ['nullable', 'string', 'max:255'],
            'notes' => ['nullable', 'string', 'max:10000'],
        ]);

        $entry = $network->entries()->updateOrCreate(
            [
                'entity_type' => $request->entity_type,
                'entity_id' => $request->entity_id,
            ],
            [
                'entity_name' => $request->entity_name,
                'color' => $request->color,
                'label' => $request->label,
                'notes' => $request->notes,
                'added_by' => $request->user()->id,
            ],
        );

        $entry->load('addedBy');

        broadcast(new IntelEntryCreated($network->id, $entry->toArray()));

        return response()->json($entry, 201);
    }

    public function update(Request $request, IntelNetwork $network, IntelEntry $entry)
    {
        $this->authorize('addEntry', $network);

        $request->validate([
            'color' => ['sometimes', 'string', 'regex:/^#[0-9A-Fa-f]{6}$/'],
            'label' => ['sometimes', 'string', 'max:255'],
            'entity_name' => ['sometimes', 'string', 'max:255'],
            'notes' => ['nullable', 'string', 'max:10000'],
        ]);

        $entry->update($request->only(['color', 'label', 'entity_name', 'notes']));
        $entry->load('addedBy');

        broadcast(new IntelEntryUpdated($network->id, $entry->toArray()));

        return response()->json($entry);
    }

    public function destroy(Request $request, IntelNetwork $network, IntelEntry $entry)
    {
        $this->authorize('addEntry', $network);

        $entryId = $entry->id;
        $entry->delete();

        broadcast(new IntelEntryDeleted($network->id, $entryId));

        return response()->json(['message' => 'Entry deleted']);
    }

    public function lookup(Request $request)
    {
        $request->validate([
            'entity_ids' => ['required', 'array'],
            'entity_ids.*' => ['integer'],
        ]);

        /** @var User $user */
        $user = $request->user();
        $accessibleIds = $user->getAccessibleIds();
        $entityIds = $request->entity_ids;

        $entries = IntelEntry::whereIn('entity_id', $entityIds)
            ->whereHas('network.accesses', function ($query) use ($accessibleIds): void {
                $query->whereIn('accessible_id', $accessibleIds)->notExpired();
            })
            ->with('network:id,name')
            ->get()
            ->map(fn (IntelEntry $entry) => [
                'id' => $entry->id,
                'intel_network_id' => $entry->intel_network_id,
                'network_name' => $entry->network->name,
                'entity_type' => $entry->entity_type,
                'entity_id' => $entry->entity_id,
                'entity_name' => $entry->entity_name,
                'color' => $entry->color,
                'label' => $entry->label,
                'notes' => $entry->notes,
            ]);

        return response()->json($entries);
    }
}
