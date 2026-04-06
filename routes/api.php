<?php

use App\Http\Controllers\ApiTokenController;
use App\Http\Controllers\EntitySearchController;
use App\Http\Controllers\IntelEntryController;
use App\Http\Controllers\IntelNetworkController;
use App\Http\Controllers\NetworkAccessController;
use App\Http\Controllers\NetworkScanController;
use App\Http\Controllers\ShareController;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Route;

Route::get('/user', function (Request $request) {
    return $request->user();
})->middleware('auth:sanctum');

Route::post('/share', [ShareController::class, 'store'])->name('api.share.store');
Route::get('/share/{code}', [ShareController::class, 'fetch'])->name('api.share.fetch');

Route::middleware('auth:sanctum')->group(function (): void {
    // API Tokens
    Route::post('/tokens', [ApiTokenController::class, 'store']);
    Route::delete('/tokens/{tokenId}', [ApiTokenController::class, 'destroy']);

    // Intel Networks
    Route::get('/networks', [IntelNetworkController::class, 'index']);
    Route::post('/networks', [IntelNetworkController::class, 'store']);
    Route::get('/networks/{network}', [IntelNetworkController::class, 'show']);
    Route::put('/networks/{network}', [IntelNetworkController::class, 'update']);
    Route::delete('/networks/{network}', [IntelNetworkController::class, 'destroy']);

    // Network Access
    Route::get('/networks/{network}/access', [NetworkAccessController::class, 'index']);
    Route::post('/networks/{network}/access', [NetworkAccessController::class, 'store']);
    Route::put('/networks/{network}/access/{access}', [NetworkAccessController::class, 'update']);
    Route::delete('/networks/{network}/access/{access}', [NetworkAccessController::class, 'destroy']);

    // Intel Entries
    Route::get('/networks/{network}/entries', [IntelEntryController::class, 'index']);
    Route::post('/networks/{network}/entries', [IntelEntryController::class, 'store']);
    Route::put('/networks/{network}/entries/{entry}', [IntelEntryController::class, 'update']);
    Route::delete('/networks/{network}/entries/{entry}', [IntelEntryController::class, 'destroy']);

    // Network Scans
    Route::get('/networks/{network}/scans', [NetworkScanController::class, 'index']);
    Route::post('/networks/{network}/scans', [NetworkScanController::class, 'store']);
    Route::get('/networks/{network}/scans/{scan}', [NetworkScanController::class, 'show']);

    // Bulk Intel Lookup
    Route::get('/intel/lookup', [IntelEntryController::class, 'lookup']);

    // Entity Search
    Route::get('/search', [EntitySearchController::class, 'search']);
});
