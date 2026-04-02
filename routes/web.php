<?php

use App\Http\Controllers\EveAuthController;
use App\Http\Controllers\ShareController;
use Illuminate\Support\Facades\Route;
use Inertia\Inertia;

Route::get('/', function () {
    return Inertia::render('Welcome');
})->name('home');

Route::get('/s/{code}', [ShareController::class, 'show'])->name('share.show');

Route::get('/eve', [EveAuthController::class, 'redirect'])->name('eve.redirect');
Route::get('/eve/callback', [EveAuthController::class, 'callback'])->name('eve.callback');
