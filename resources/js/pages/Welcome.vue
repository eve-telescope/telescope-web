<script setup lang="ts">
import { Head, usePage } from '@inertiajs/vue3';
import { computed, onMounted, ref } from 'vue';

const page = usePage();
const appVersion = computed(() => page.props.appVersion ?? 'v0.1.0');

const stars = ref<{ x: number; y: number; size: number; opacity: number; delay: number }[]>([]);

onMounted(() => {
    for (let i = 0; i < 100; i++) {
        stars.value.push({
            x: Math.random() * 100,
            y: Math.random() * 100,
            size: Math.random() * 2 + 0.5,
            opacity: Math.random() * 0.8 + 0.2,
            delay: Math.random() * 3,
        });
    }
});
</script>

<template>
    <Head title="EVE Online Intel Tool" />

    <div class="relative min-h-screen overflow-hidden bg-eve-bg-0">
        <!-- Animated starfield background -->
        <div class="absolute inset-0 overflow-hidden">
            <div
                v-for="(star, i) in stars"
                :key="i"
                class="absolute animate-pulse rounded-full bg-white"
                :style="{
                    left: `${star.x}%`,
                    top: `${star.y}%`,
                    width: `${star.size}px`,
                    height: `${star.size}px`,
                    opacity: star.opacity,
                    animationDelay: `${star.delay}s`,
                    animationDuration: '3s',
                }"
            />
        </div>

        <!-- Gradient overlays -->
        <div class="absolute inset-0 bg-gradient-to-b from-eve-bg-0 via-transparent to-eve-bg-0 opacity-60" />
        <div class="absolute inset-0 bg-gradient-to-r from-eve-cyan/5 via-transparent to-eve-orange/5" />

        <!-- Scan line effect -->
        <div class="pointer-events-none absolute inset-0 overflow-hidden">
            <div
                class="absolute inset-0 bg-[repeating-linear-gradient(0deg,transparent,transparent_2px,rgba(0,212,255,0.03)_2px,rgba(0,212,255,0.03)_4px)]"
            />
        </div>

        <!-- Main content -->
        <div class="relative z-10 flex min-h-screen flex-col items-center justify-center px-6 py-20">
            <!-- Logo / Title -->
            <div class="mb-12 text-center">
                <div class="relative inline-block">
                    <!-- Glow effect -->
                    <div class="absolute -inset-4 rounded-full bg-eve-cyan/20 blur-2xl" />

                    <!-- Telescope icon -->
                    <svg
                        class="relative mx-auto mb-6 h-24 w-24 drop-shadow-[0_0_20px_rgba(0,212,255,0.5)]"
                        viewBox="0 0 24 24"
                        xmlns="http://www.w3.org/2000/svg"
                    >
                        <rect x="0" y="0" width="24" height="24" rx="4.5" fill="#12151a" />
                        <g stroke="#00d4ff" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none">
                            <circle cx="12" cy="12" r="7" />
                            <line x1="19" x2="16" y1="12" y2="12" />
                            <line x1="8" x2="5" y1="12" y2="12" />
                            <line x1="12" x2="12" y1="8" y2="5" />
                            <line x1="12" x2="12" y1="19" y2="16" />
                        </g>
                    </svg>
                </div>

                <h1 class="mb-3 text-5xl font-bold tracking-wider text-eve-text-1 drop-shadow-[0_0_30px_rgba(0,212,255,0.3)]">TELESCOPE</h1>
                <p class="text-lg tracking-widest text-eve-cyan">EVE ONLINE INTEL TOOL</p>
            </div>

            <!-- App Screenshot -->
            <div class="relative mb-16 w-full max-w-5xl">
                <div class="absolute -inset-1 rounded-lg bg-gradient-to-r from-eve-cyan/20 via-eve-orange/10 to-eve-cyan/20 opacity-50 blur-lg" />
                <div class="relative overflow-hidden rounded-lg border border-eve-border shadow-2xl shadow-eve-cyan/10">
                    <img src="/app.png" alt="Telescope App Screenshot" class="w-full" />
                </div>
            </div>

            <!-- Feature cards -->
            <div class="mb-12 grid max-w-4xl gap-6 md:grid-cols-3">
                <div
                    class="group relative overflow-hidden rounded-lg border border-eve-border bg-eve-bg-1/80 p-6 backdrop-blur-sm transition-all hover:border-eve-cyan/50 hover:shadow-[0_0_30px_rgba(0,212,255,0.1)]"
                >
                    <div class="absolute -top-4 -right-4 h-24 w-24 rounded-full bg-eve-cyan/10 blur-2xl transition-all group-hover:bg-eve-cyan/20" />
                    <div class="relative">
                        <div class="mb-3 text-eve-cyan">
                            <svg class="h-8 w-8" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    d="M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zm8.25 2.25a2.625 2.625 0 11-5.25 0 2.625 2.625 0 015.25 0z"
                                />
                            </svg>
                        </div>
                        <h3 class="mb-2 text-sm font-semibold tracking-wider text-eve-text-1">INSTANT LOOKUP</h3>
                        <p class="text-xs leading-relaxed text-eve-text-2">Paste local chat, get instant threat assessment from zKillboard data</p>
                    </div>
                </div>

                <div
                    class="group relative overflow-hidden rounded-lg border border-eve-border bg-eve-bg-1/80 p-6 backdrop-blur-sm transition-all hover:border-eve-orange/50 hover:shadow-[0_0_30px_rgba(255,107,0,0.1)]"
                >
                    <div
                        class="absolute -top-4 -right-4 h-24 w-24 rounded-full bg-eve-orange/10 blur-2xl transition-all group-hover:bg-eve-orange/20"
                    />
                    <div class="relative">
                        <div class="mb-3 text-eve-orange">
                            <svg class="h-8 w-8" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 013 19.875v-6.75zM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V8.625zM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V4.125z"
                                />
                            </svg>
                        </div>
                        <h3 class="mb-2 text-sm font-semibold tracking-wider text-eve-text-1">THREAT ANALYSIS</h3>
                        <p class="text-xs leading-relaxed text-eve-text-2">See kill history, favorite ships, activity patterns, and danger ratings</p>
                    </div>
                </div>

                <div
                    class="group relative overflow-hidden rounded-lg border border-eve-border bg-eve-bg-1/80 p-6 backdrop-blur-sm transition-all hover:border-eve-green/50 hover:shadow-[0_0_30px_rgba(0,255,136,0.1)]"
                >
                    <div
                        class="absolute -top-4 -right-4 h-24 w-24 rounded-full bg-eve-green/10 blur-2xl transition-all group-hover:bg-eve-green/20"
                    />
                    <div class="relative">
                        <div class="mb-3 text-eve-green">
                            <svg class="h-8 w-8" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    d="M7.217 10.907a2.25 2.25 0 100 2.186m0-2.186c.18.324.283.696.283 1.093s-.103.77-.283 1.093m0-2.186l9.566-5.314m-9.566 7.5l9.566 5.314m0 0a2.25 2.25 0 103.935 2.186 2.25 2.25 0 00-3.935-2.186zm0-12.814a2.25 2.25 0 103.933-2.185 2.25 2.25 0 00-3.933 2.185z"
                                />
                            </svg>
                        </div>
                        <h3 class="mb-2 text-sm font-semibold tracking-wider text-eve-text-1">SHARE INTEL</h3>
                        <p class="text-xs leading-relaxed text-eve-text-2">Share scans via link — your corp mates can open results instantly</p>
                    </div>
                </div>
            </div>

            <!-- Download section -->
            <div class="text-center">
                <p class="mb-6 text-xs tracking-widest text-eve-text-3">AVAILABLE FOR</p>
                <div class="flex flex-wrap justify-center gap-4">
                    <a
                        href="https://github.com/eve-telescope/telescope-app/releases/latest"
                        target="_blank"
                        class="group relative overflow-hidden rounded-lg border border-eve-border bg-eve-bg-2 px-8 py-4 transition-all hover:border-eve-cyan hover:shadow-[0_0_30px_rgba(0,212,255,0.2)]"
                    >
                        <div
                            class="absolute inset-0 bg-gradient-to-r from-eve-cyan/0 via-eve-cyan/5 to-eve-cyan/0 opacity-0 transition-opacity group-hover:opacity-100"
                        />
                        <div class="relative flex items-center gap-3">
                            <svg class="h-6 w-6 text-eve-text-2 group-hover:text-eve-cyan" viewBox="0 0 24 24" fill="currentColor">
                                <path
                                    d="M22 17.607c-.786 2.28-3.139 6.317-5.563 6.361-1.608.031-2.125-.953-3.963-.953-1.837 0-2.412.923-3.932.983-2.572.099-6.542-5.827-6.542-10.995 0-4.747 3.308-7.1 6.198-7.143 1.55-.028 3.014 1.045 3.959 1.045.949 0 2.727-1.29 4.596-1.101.782.033 2.979.315 4.389 2.377-3.741 2.442-3.158 7.549.858 9.426zm-5.222-17.607c-2.826.114-5.132 3.079-4.81 5.531 2.612.203 5.118-2.725 4.81-5.531z"
                                />
                            </svg>
                            <div class="text-left">
                                <div class="text-[10px] tracking-wider text-eve-text-3">DOWNLOAD FOR</div>
                                <div class="text-sm font-semibold tracking-wider text-eve-text-1">macOS</div>
                            </div>
                        </div>
                    </a>

                    <a
                        href="https://github.com/eve-telescope/telescope-app/releases/latest"
                        target="_blank"
                        class="group relative overflow-hidden rounded-lg border border-eve-border bg-eve-bg-2 px-8 py-4 transition-all hover:border-eve-cyan hover:shadow-[0_0_30px_rgba(0,212,255,0.2)]"
                    >
                        <div
                            class="absolute inset-0 bg-gradient-to-r from-eve-cyan/0 via-eve-cyan/5 to-eve-cyan/0 opacity-0 transition-opacity group-hover:opacity-100"
                        />
                        <div class="relative flex items-center gap-3">
                            <svg class="h-6 w-6 text-eve-text-2 group-hover:text-eve-cyan" viewBox="0 0 24 24" fill="currentColor">
                                <path
                                    d="M0 3.449L9.75 2.1v9.451H0m10.949-9.602L24 0v11.4H10.949M0 12.6h9.75v9.451L0 20.699M10.949 12.6H24V24l-12.9-1.801"
                                />
                            </svg>
                            <div class="text-left">
                                <div class="text-[10px] tracking-wider text-eve-text-3">DOWNLOAD FOR</div>
                                <div class="text-sm font-semibold tracking-wider text-eve-text-1">Windows</div>
                            </div>
                        </div>
                    </a>

                    <a
                        href="https://github.com/eve-telescope/telescope-app/releases/latest"
                        target="_blank"
                        class="group relative overflow-hidden rounded-lg border border-eve-border bg-eve-bg-2 px-8 py-4 transition-all hover:border-eve-cyan hover:shadow-[0_0_30px_rgba(0,212,255,0.2)]"
                    >
                        <div
                            class="absolute inset-0 bg-gradient-to-r from-eve-cyan/0 via-eve-cyan/5 to-eve-cyan/0 opacity-0 transition-opacity group-hover:opacity-100"
                        />
                        <div class="relative flex items-center gap-3">
                            <svg class="h-6 w-6 text-eve-text-2 group-hover:text-eve-cyan" viewBox="0 0 32 32" fill="currentColor">
                                <path
                                    d="M14.923 8.080c-0.025 0.072-0.141 0.061-0.207 0.082-0.059 0.031-0.107 0.085-0.175 0.085-0.062 0-0.162-0.025-0.17-0.085-0.012-0.082 0.11-0.166 0.187-0.166 0.050-0.024 0.108-0.037 0.169-0.037 0.056 0 0.109 0.011 0.157 0.032l-0.003-0.001c0.022 0.009 0.038 0.030 0.038 0.055 0 0.003-0 0.005-0.001 0.008l0-0v0.025h0.004zM15.611 8.080v-0.027c-0.008-0.025 0.016-0.052 0.036-0.062 0.046-0.020 0.1-0.032 0.157-0.032 0.061 0 0.119 0.014 0.17 0.038l-0.002-0.001c0.079 0 0.2 0.084 0.187 0.169-0.007 0.061-0.106 0.082-0.169 0.082-0.069 0-0.115-0.054-0.176-0.085-0.065-0.023-0.182-0.010-0.204-0.081zM16.963 10.058c-0.532 0.337-1.161 0.574-1.835 0.666l-0.024 0.003c-0.606-0.035-1.157-0.248-1.607-0.588l0.007 0.005c-0.192-0.167-0.35-0.335-0.466-0.419-0.205-0.167-0.18-0.416-0.092-0.416 0.136 0.020 0.161 0.167 0.249 0.25 0.12 0.082 0.269 0.25 0.45 0.416 0.397 0.328 0.899 0.541 1.45 0.583l0.009 0.001c0.654-0.057 1.249-0.267 1.763-0.592l-0.016 0.010c0.244-0.169 0.556-0.417 0.81-0.584 0.195-0.17 0.186-0.334 0.349-0.334 0.16 0.020 0.043 0.167-0.184 0.415-0.246 0.188-0.527 0.381-0.818 0.56l-0.044 0.025zM8.017 21.397h0.012c0.069 0 0.137 0.007 0.203 0.019l-0.007-0.001c0.544 0.14 0.992 0.478 1.273 0.931l0.005 0.009 1.137 2.079 0.004 0.004c0.457 0.773 0.948 1.442 1.497 2.059l-0.011-0.013c0.49 0.52 0.82 1.196 0.909 1.946l0.002 0.016v0.008c-0.012 0.817-0.613 1.491-1.396 1.616l-0.009 0.001c-0.2 0.031-0.432 0.048-0.667 0.048-0.857 0-1.659-0.233-2.347-0.64l0.021 0.012c-1.053-0.441-2.275-0.714-3.555-0.752l-0.015-0c-0.372-0.025-0.696-0.215-0.901-0.496l-0.002-0.003c-0.054-0.174-0.085-0.374-0.085-0.582 0-0.35 0.088-0.679 0.244-0.966l-0.005 0.011v-0.005l0.003-0.004c0.041-0.188 0.065-0.405 0.065-0.627 0-0.274-0.036-0.539-0.104-0.791l0.005 0.021c-0.041-0.15-0.065-0.323-0.065-0.502 0-0.242 0.043-0.473 0.123-0.687l-0.004 0.014c0.2-0.417 0.495-0.5 0.862-0.666 0.438-0.133 0.819-0.334 1.151-0.593l-0.008 0.006h0.002v-0.003c0.32-0.335 0.556-0.751 0.835-1.047 0.195-0.249 0.492-0.41 0.827-0.42l0.002-0zM21.531 21.336c-0.001 0.017-0.001 0.038-0.001 0.059 0 0.743 0.449 1.381 1.091 1.658l0.012 0.005c0.048 0.003 0.104 0.005 0.16 0.005 0.831 0 1.575-0.371 2.075-0.957l0.003-0.004 0.264-0.012c0.053-0.008 0.114-0.012 0.176-0.012 0.341 0 0.652 0.132 0.883 0.348l-0.001-0.001 0.004 0.004c0.249 0.301 0.422 0.673 0.487 1.082l0.002 0.013c0.055 0.505 0.238 0.96 0.517 1.34l-0.005-0.008c0.416 0.356 0.705 0.85 0.793 1.411l0.002 0.013 0.004-0.009v0.022l-0.004-0.015c-0.019 0.327-0.231 0.495-0.622 0.744-1.184 0.497-2.201 1.158-3.077 1.968l0.007-0.006c-0.608 0.792-1.501 1.339-2.523 1.486l-0.021 0.002c-0.074 0.010-0.16 0.016-0.247 0.016-0.768 0-1.428-0.464-1.716-1.126l-0.005-0.012-0.006-0.004c-0.093-0.286-0.146-0.615-0.146-0.956 0-0.416 0.079-0.813 0.224-1.178l-0.008 0.022c0.234-0.668 0.435-1.466 0.568-2.288l0.011-0.083c0.016-0.812 0.104-1.593 0.258-2.35l-0.014 0.083c0.085-0.518 0.381-0.954 0.794-1.225l0.007-0.004 0.056-0.027zM18.8 10.142c0.6 2.147 1.339 4.002 2.247 5.757l-0.079-0.167c0.613 1.090 1.090 2.355 1.363 3.695l0.014 0.084c0.009-0 0.020-0 0.031-0 0.217 0 0.427 0.029 0.627 0.084l-0.017-0.004c0.11-0.395 0.173-0.848 0.173-1.316 0-1.426-0.587-2.716-1.533-3.639l-0.001-0.001c-0.275-0.25-0.29-0.419-0.154-0.419 0.971 0.91 1.689 2.078 2.045 3.394l0.012 0.051c0.089 0.329 0.14 0.707 0.14 1.097 0 0.351-0.041 0.693-0.119 1.020l0.006-0.030c0.074 0.038 0.16 0.067 0.251 0.083l0.006 0.001c1.29 0.667 1.766 1.172 1.537 1.921v-0.054c-0.075-0.004-0.15 0-0.225 0h-0.020c0.189-0.584-0.227-1.031-1.331-1.53-1.143-0.5-2.057-0.42-2.212 0.581-0.011 0.049-0.019 0.106-0.022 0.165l-0 0.003c-0.073 0.030-0.16 0.058-0.25 0.078l-0.011 0.002c-0.508 0.336-0.87 0.859-0.989 1.469l-0.002 0.014c-0.148 0.695-0.241 1.5-0.256 2.323l-0 0.012v0.004c-0.091 0.637-0.23 1.207-0.418 1.753l0.020-0.066c-0.983 0.804-2.251 1.29-3.634 1.29-1.13 0-2.184-0.325-3.073-0.887l0.024 0.014c-0.146-0.253-0.313-0.472-0.503-0.667l0.001 0.001c-0.097-0.16-0.211-0.297-0.342-0.415l-0.002-0.001c0.207-0 0.407-0.031 0.596-0.088l-0.015 0.004c0.18-0.085 0.318-0.232 0.391-0.412l0.002-0.005c0.018-0.093 0.029-0.199 0.029-0.308 0-0.445-0.175-0.848-0.461-1.146l0.001 0.001c-0.619-0.761-1.359-1.395-2.196-1.88l-0.038-0.020c-0.671-0.388-1.179-0.995-1.43-1.722l-0.007-0.022c-0.093-0.318-0.147-0.684-0.147-1.062 0-0.353 0.047-0.695 0.134-1.021l-0.006 0.027c0.377-1.314 0.921-2.461 1.62-3.496l-0.028 0.043c0.134-0.081 0.046 0.169-0.51 1.217-0.474 0.713-0.757 1.59-0.757 2.533 0 0.84 0.224 1.627 0.616 2.306l-0.012-0.022c0.052-1.309 0.345-2.537 0.834-3.659l-0.025 0.065c1.055-1.902 1.854-4.111 2.275-6.452l0.020-0.131c0.060 0.045 0.271 0.169 0.361 0.252 0.272 0.166 0.475 0.416 0.737 0.581 0.267 0.26 0.633 0.42 1.035 0.42 0.021 0 0.042-0 0.063-0.001l-0.003 0c0.049 0.004 0.094 0.008 0.137 0.008 0.459-0.009 0.887-0.132 1.259-0.342l-0.013 0.007c0.362-0.167 0.65-0.417 0.925-0.5h0.006c0.535-0.145 0.983-0.454 1.3-0.869l0.004-0.006zM15.301 7.465c0.003 0 0.006-0 0.009-0 0.569 0 1.094 0.187 1.517 0.503l-0.007-0.005c0.378 0.234 0.814 0.433 1.275 0.574l0.040 0.010h0.004c0.246 0.11 0.449 0.281 0.594 0.494l0.003 0.005v-0.164c0.046 0.092 0.074 0.201 0.074 0.316 0 0.098-0.020 0.191-0.055 0.276l0.002-0.005c-0.288 0.507-0.755 0.884-1.313 1.048l-0.016 0.004v0.002c-0.335 0.169-0.626 0.416-0.968 0.581-0.35 0.21-0.771 0.334-1.222 0.334-0.015 0-0.030-0-0.045-0l0.002 0c-0.022 0.001-0.048 0.002-0.074 0.002-0.174 0-0.342-0.031-0.496-0.089l0.010 0.003c-0.159-0.087-0.29-0.169-0.417-0.257l0.014 0.010c-0.227-0.199-0.477-0.39-0.739-0.565l-0.026-0.016v-0.006h-0.006c-0.375-0.199-0.67-0.504-0.852-0.876l-0.005-0.012c-0.027-0.067-0.042-0.145-0.042-0.226 0-0.218 0.112-0.41 0.281-0.522l0.002-0.001c0.28-0.169 0.475-0.339 0.604-0.42 0.13-0.092 0.179-0.127 0.22-0.164h0.002v-0.004c0.268-0.339 0.623-0.599 1.032-0.746l0.016-0.005c0.174-0.050 0.374-0.079 0.581-0.081h0.001zM13.589 5.333h0.045c0.188 0.004 0.361 0.067 0.501 0.17l-0.002-0.002c0.179 0.159 0.325 0.352 0.425 0.570l0.004 0.011c0.113 0.245 0.183 0.530 0.191 0.830l0 0.003v0.005c0.004 0.046 0.006 0.099 0.006 0.152 0 0.063-0.003 0.126-0.009 0.188l0.001-0.008v0.1c-0.037 0.009-0.070 0.022-0.104 0.030-0.191 0.079-0.352 0.163-0.505 0.258l0.014-0.008c0.008-0.055 0.012-0.118 0.012-0.182 0-0.053-0.003-0.106-0.009-0.158l0.001 0.006v-0.019c-0.018-0.154-0.054-0.295-0.107-0.428l0.004 0.011c-0.041-0.132-0.113-0.244-0.207-0.333l-0-0c-0.055-0.050-0.128-0.081-0.209-0.081-0.007 0-0.014 0-0.021 0.001l0.001-0h-0.026c-0.103 0.011-0.189 0.075-0.232 0.163l-0.001 0.002c-0.077 0.093-0.130 0.208-0.150 0.334l-0 0.004c-0.023 0.086-0.035 0.185-0.035 0.287 0 0.044 0.002 0.088 0.007 0.131l-0-0.005v0.019c0.016 0.154 0.052 0.296 0.104 0.428l-0.004-0.011c0.042 0.132 0.113 0.245 0.207 0.335l0 0c0.012 0.012 0.026 0.022 0.042 0.030l0.001 0c-0.083 0.053-0.155 0.109-0.221 0.171l0.001-0.001c-0.045 0.040-0.100 0.070-0.161 0.084l-0.003 0.001c-0.123-0.147-0.237-0.312-0.335-0.486l-0.008-0.016c-0.113-0.245-0.183-0.529-0.194-0.830l-0-0.004c-0.004-0.048-0.006-0.104-0.006-0.161 0-0.241 0.039-0.473 0.110-0.690l-0.004 0.016c0.074-0.258 0.195-0.481 0.356-0.671l-0.002 0.003c0.127-0.150 0.313-0.245 0.522-0.250h0.001zM17.291 5.259h0.016c0.001 0 0.002 0 0.004 0 0.275 0 0.527 0.093 0.729 0.249l-0.003-0.002c0.229 0.177 0.413 0.400 0.542 0.655l0.005 0.011c0.121 0.266 0.196 0.575 0.207 0.901l0 0.004c0-0.025 0.007-0.050 0.007-0.075v0.131l-0.005-0.026-0.005-0.030c-0.003 0.320-0.071 0.622-0.193 0.897l0.006-0.014c-0.062 0.163-0.152 0.303-0.266 0.419l0-0c-0.030-0.018-0.067-0.035-0.104-0.050l-0.006-0.002c-0.135-0.042-0.253-0.099-0.360-0.169l0.005 0.003c-0.077-0.032-0.169-0.060-0.264-0.081l-0.011-0.002c0.081-0.076 0.156-0.157 0.225-0.243l0.004-0.005c0.063-0.148 0.102-0.319 0.110-0.499l0-0.003v-0.025c0-0.008 0-0.016 0-0.025 0-0.170-0.028-0.333-0.080-0.485l0.003 0.011c-0.063-0.159-0.140-0.296-0.232-0.421l0.004 0.005c-0.087-0.088-0.202-0.148-0.331-0.165l-0.003-0h-0.020c-0.001 0-0.003-0-0.004-0-0.132 0-0.250 0.065-0.322 0.164l-0.001 0.001c-0.116 0.113-0.204 0.253-0.254 0.410l-0.002 0.007c-0.063 0.147-0.104 0.318-0.112 0.496l-0 0.003v0.024c0.002 0.120 0.011 0.236 0.027 0.349l-0.002-0.015c-0.241-0.084-0.547-0.169-0.759-0.252-0.012-0.073-0.020-0.159-0.022-0.247l-0-0.003v-0.025c-0.001-0.020-0.001-0.043-0.001-0.066 0-0.324 0.069-0.631 0.194-0.908l-0.006 0.014c0.106-0.279 0.293-0.508 0.532-0.663l0.005-0.003c0.204-0.156 0.462-0.250 0.742-0.250h0zM16.63 1.004c-0.194 0-0.394 0.010-0.600 0.026-5.281 0.416-3.880 6.007-3.961 7.870-0.050 1.426-0.534 2.729-1.325 3.792l0.013-0.018c-1.407 1.602-2.555 3.474-3.351 5.523l-0.043 0.127c-0.258 0.685-0.408 1.476-0.408 2.302 0 0.285 0.018 0.566 0.052 0.841l-0.003-0.033c-0.056 0.046-0.103 0.102-0.136 0.166l-0.001 0.003c-0.325 0.335-0.562 0.750-0.829 1.048-0.283 0.217-0.615 0.388-0.975 0.494l-0.021 0.005c-0.464 0.139-0.842 0.442-1.075 0.841l-0.005 0.009c-0.104 0.212-0.165 0.461-0.165 0.725 0 0.010 0 0.019 0 0.029l-0-0.001c0.002 0.238 0.026 0.469 0.073 0.693l-0.004-0.023c0.056 0.219 0.088 0.471 0.088 0.730 0 0.170-0.014 0.337-0.041 0.500l0.002-0.018c-0.167 0.313-0.264 0.685-0.264 1.080 0 0.278 0.048 0.544 0.137 0.791l-0.005-0.016c0.273 0.388 0.686 0.662 1.164 0.749l0.011 0.002c1.274 0.107 2.451 0.373 3.561 0.780l-0.094-0.030c0.698 0.415 1.539 0.660 2.436 0.660 0.294 0 0.582-0.026 0.862-0.077l-0.029 0.004c0.667-0.151 1.211-0.586 1.504-1.169l0.006-0.013c0.734-0.004 1.537-0.336 2.824-0.417 0.873-0.072 1.967 0.334 3.220 0.250 0.037 0.159 0.086 0.298 0.148 0.429l-0.006-0.013 0.004 0.004c0.384 0.804 1.190 1.350 2.124 1.350 0.081 0 0.161-0.004 0.240-0.012l-0.010 0.001c1.151-0.170 2.139-0.768 2.813-1.623l0.007-0.009c0.843-0.768 1.827-1.401 2.905-1.853l0.067-0.025c0.432-0.191 0.742-0.585 0.810-1.059l0.001-0.007c-0.059-0.694-0.392-1.299-0.888-1.716l-0.004-0.003v-0.121l-0.004-0.004c-0.214-0.330-0.364-0.722-0.421-1.142l-0.002-0.015c-0.053-0.513-0.278-0.966-0.615-1.307l0 0h-0.004c-0.074-0.067-0.154-0.084-0.235-0.169-0.066-0.047-0.148-0.076-0.237-0.080l-0.001-0c0.195-0.602 0.308-1.294 0.308-2.013 0-0.940-0.193-1.835-0.541-2.647l0.017 0.044c-0.704-1.672-1.619-3.111-2.732-4.369l0.014 0.017c-1.105-1.082-1.828-2.551-1.948-4.187l-0.001-0.021c0.033-2.689 0.295-7.664-4.429-7.671z"
                                />
                            </svg>
                            <div class="text-left">
                                <div class="text-[10px] tracking-wider text-eve-text-3">DOWNLOAD FOR</div>
                                <div class="text-sm font-semibold tracking-wider text-eve-text-1">Linux</div>
                            </div>
                        </div>
                    </a>
                </div>
            </div>

            <!-- Version info -->
            <div class="mt-12 text-center">
                <p class="text-xs text-eve-text-3">{{ appVersion }} • Open Source</p>
                <a
                    href="https://github.com/eve-telescope/telescope-app"
                    target="_blank"
                    class="mt-2 inline-flex items-center gap-2 text-xs text-eve-text-3 transition-colors hover:text-eve-cyan"
                >
                    <svg class="h-4 w-4" fill="currentColor" viewBox="0 0 24 24">
                        <path
                            d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"
                        />
                    </svg>
                    View on GitHub
                </a>
            </div>
        </div>

        <!-- Bottom glow -->
        <div class="absolute bottom-0 left-1/2 h-px w-1/2 -translate-x-1/2 bg-gradient-to-r from-transparent via-eve-cyan/50 to-transparent" />
    </div>
</template>
