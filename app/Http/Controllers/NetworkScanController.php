<?php

namespace App\Http\Controllers;

use App\DTOs\NetworkScanData;
use App\Events\ScanShared;
use App\Models\IntelNetwork;
use App\Models\NetworkScan;
use Illuminate\Http\Request;

class NetworkScanController extends Controller
{
    public function index(Request $request, IntelNetwork $network)
    {
        $this->authorize('view', $network);

        $scans = $network->scans()
            ->with('user')
            ->latest()
            ->paginate(20);

        $scans->getCollection()->transform(
            fn (NetworkScan $scan) => NetworkScanData::fromModel($scan)->toArray()
        );

        return response()->json($scans);
    }

    public function show(Request $request, IntelNetwork $network, NetworkScan $scan)
    {
        $this->authorize('view', $network);

        abort_unless($scan->intel_network_id === $network->id, 404);

        $scan->load('user');

        return response()->json(NetworkScanData::fromModel($scan)->toArray());
    }

    public function store(Request $request, IntelNetwork $network)
    {
        $this->authorize('addEntry', $network);

        $request->validate([
            'scan_type' => ['required', 'string', 'in:local,dscan'],
            'raw_text' => ['required', 'string', 'max:500000'],
            'solar_system' => ['nullable', 'string', 'max:255'],
        ]);

        $scan = $network->scans()->create([
            'user_id' => $request->user()->id,
            'scan_type' => $request->scan_type,
            'raw_text' => $request->raw_text,
            'solar_system' => $request->solar_system,
        ]);

        broadcast(new ScanShared($network->id, $scan->id));

        $scan->load('user');

        return response()->json(NetworkScanData::fromModel($scan)->toArray(), 201);
    }
}
