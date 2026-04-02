<?php

namespace App\Http\Controllers;

use App\Services\EveAuthService;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Auth;
use Inertia\Inertia;
use Laravel\Socialite\Facades\Socialite;

class EveAuthController extends Controller
{
    public function redirect(Request $request)
    {
        if ($request->has('desktop')) {
            session(['desktop_auth' => true]);
        }

        return Socialite::driver('eveonline')->redirect();
    }

    public function callback(EveAuthService $authService)
    {
        $user = $authService->getUser();

        if (session()->pull('desktop_auth')) {
            $token = $user->createToken('Telescope Desktop')->plainTextToken;
            $deepLink = "telescope://auth?token={$token}";

            return Inertia::render('AuthToken', [
                'token' => $token,
                'deepLink' => $deepLink,
                'characterName' => $user->character_name,
            ]);
        }

        Auth::login($user, remember: true);

        return redirect()->intended('/');
    }
}
