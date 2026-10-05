<!DOCTYPE html>
<html lang="{{ str_replace('_', '-', app()->getLocale()) }}"  @class(['dark' => ($appearance ?? 'system') == 'dark'])>
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">

        <title inertia>{{ config('app.name', 'Telescope') }}</title>

        <meta name="description" content="Intel tool for EVE Online. Instant pilot lookups and danger ratings.">
        <meta name="theme-color" content="#0a0a0b">

        <meta property="og:type" content="website">
        <meta property="og:site_name" content="Telescope">
        <meta property="og:title" content="Telescope">
        <meta property="og:description" content="Intel tool for EVE Online. Instant pilot lookups and danger ratings.">
        <meta property="og:url" content="{{ url()->current() }}">
        <meta property="og:image" content="{{ asset('og-image.png') }}">
        <meta property="og:image:width" content="1200">
        <meta property="og:image:height" content="630">
        <meta name="twitter:card" content="summary_large_image">
        <meta name="twitter:image" content="{{ asset('og-image.png') }}">

        <link rel="icon" href="/favicon.ico" sizes="48x48">
        <link rel="icon" href="/favicon-32.png" sizes="32x32" type="image/png">
        <link rel="icon" href="/favicon-16.png" sizes="16x16" type="image/png">
        <link rel="apple-touch-icon" href="/apple-touch-icon.png">
        <link rel="manifest" href="/site.webmanifest">

        <link rel="preconnect" href="https://fonts.bunny.net">
        <link href="https://fonts.bunny.net/css?family=inter:400,500,600,700|jetbrains-mono:400,500,600" rel="stylesheet" />

        @vite(['resources/js/app.ts'])
        <x-inertia::head />
    </head>
    <body class="font-sans antialiased">
        <x-inertia::app />
    </body>
</html>
