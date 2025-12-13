<?php

use App\Http\Controllers\ShareController;
use Illuminate\Support\Facades\Route;
use Inertia\Inertia;

Route::get('/', function () {
    return Inertia::render('Welcome');
})->name('home');

Route::get('/s/{code}', [ShareController::class, 'show'])->name('share.show');
