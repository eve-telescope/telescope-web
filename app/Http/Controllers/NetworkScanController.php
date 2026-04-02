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

        $scan->load('user');

        $data = NetworkScanData::fromModel($scan)->toArray();

        broadcast(new ScanShared($network->id, $data))->toOthers();

        return response()->json($data, 201);
    }
}
