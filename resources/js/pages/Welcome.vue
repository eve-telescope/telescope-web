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
                            <svg class="h-6 w-6 text-eve-text-2 group-hover:text-eve-cyan" viewBox="0 0 24 24" fill="currentColor">
                                <path
                                    d="M12.504 0c-.155 0-.311.001-.465.003-.658.014-1.317.066-1.97.157-.653.092-1.296.224-1.924.396-.628.172-1.24.384-1.833.635-.593.25-1.166.54-1.713.867-.548.327-1.068.692-1.558 1.093-.49.4-.948.837-1.37 1.308-.422.47-.808.975-1.155 1.51-.347.535-.654 1.099-.919 1.688-.265.589-.486 1.203-.662 1.836-.176.633-.306 1.284-.388 1.948-.082.664-.115 1.34-.099 2.021.016.68.081 1.365.193 2.046.112.68.271 1.354.475 2.017.204.663.453 1.313.745 1.942.292.63.626 1.238.999 1.819.374.58.787 1.133 1.236 1.651.449.519.933 1.002 1.449 1.447.516.445 1.064.851 1.638 1.215.575.364 1.175.686 1.795.962.62.277 1.261.508 1.917.691.656.183 1.326.319 2.004.406.678.087 1.364.125 2.051.113.687-.012 1.373-.074 2.053-.184.68-.11 1.351-.268 2.007-.472.656-.204 1.297-.454 1.917-.748.62-.294 1.219-.631 1.79-1.01.572-.378 1.114-.796 1.623-1.25.509-.453.983-.942 1.42-1.463.436-.52.833-1.072 1.187-1.65.354-.578.665-1.181.93-1.802.266-.621.485-1.261.656-1.914.171-.653.294-1.32.368-1.993.074-.673.098-1.352.073-2.031-.025-.679-.1-1.356-.224-2.024-.123-.668-.295-1.325-.513-1.965-.218-.64-.483-1.262-.791-1.859-.309-.597-.661-1.168-1.054-1.707-.393-.539-.825-1.045-1.293-1.513-.468-.467-.972-.897-1.507-1.285-.536-.388-1.1-.734-1.687-1.034-.587-.3-1.197-.554-1.823-.76-.626-.205-1.268-.361-1.919-.467C13.854.06 13.18.012 12.504 0zm-.124 2.36c.584.005 1.166.056 1.741.151.575.095 1.141.235 1.692.418.55.183 1.084.409 1.596.675.513.266 1.002.571 1.462.913.46.342.89.72 1.287 1.129.397.409.759.85 1.082 1.317.323.467.607.96.848 1.473.242.513.44 1.047.594 1.594.154.546.262 1.107.325 1.674.062.567.078 1.14.047 1.712-.031.571-.108 1.14-.231 1.698-.123.559-.29 1.107-.502 1.637-.212.53-.466 1.04-.762 1.525-.296.485-.631.943-1.003 1.369-.372.426-.778.82-1.215 1.176-.437.357-.904.677-1.393.958-.49.28-1.002.521-1.532.72-.53.199-1.076.355-1.632.468-.556.113-1.121.181-1.69.205-.568.024-1.139.003-1.705-.061-.566-.064-1.125-.172-1.672-.323-.547-.15-1.08-.343-1.593-.577-.514-.234-1.006-.508-1.473-.82-.467-.312-.908-.661-1.317-1.043-.409-.383-.785-.799-1.124-1.244-.339-.445-.64-.918-.899-1.413-.26-.496-.477-1.013-.65-1.545-.173-.532-.302-1.08-.385-1.635-.083-.556-.12-1.119-.111-1.683.009-.564.065-1.126.167-1.679.102-.553.25-1.096.442-1.622.192-.526.429-1.035.707-1.518.278-.484.597-.942.954-1.368.357-.427.75-.821 1.175-1.179.425-.358.881-.68 1.362-.961.481-.28.985-.52 1.507-.716.521-.196 1.059-.348 1.607-.455.548-.107 1.105-.168 1.663-.183.14-.004.279-.005.418-.004z"
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
