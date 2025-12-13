<?php

use App\Http\Controllers\ShareController;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Route;

Route::get('/user', function (Request $request) {
    return $request->user();
})->middleware('auth:sanctum');

Route::post('/share', [ShareController::class, 'store'])->name('api.share.store');
Route::get('/share/{code}', [ShareController::class, 'fetch'])->name('api.share.fetch');
