<script setup lang="ts">
import { Head } from '@inertiajs/vue3';
import { computed, onMounted, ref } from 'vue';

interface ShareData {
    code: string;
    pilots: string[];
    pilotCount: number;
    views: number;
    createdAt: string;
}

const props = defineProps<{
    share: ShareData;
}>();

const deepLinkUrl = computed(() => `telescope://s/${props.share.code}`);
const isOpening = ref(false);
const showFallback = ref(false);
const pilotsCopied = ref(false);

async function copyPilots() {
    await navigator.clipboard.writeText(props.share.pilots.join('\n'));
    pilotsCopied.value = true;
    setTimeout(() => {
        pilotsCopied.value = false;
    }, 2000);
}

function openInApp() {
    isOpening.value = true;
    window.location.href = deepLinkUrl.value;

    setTimeout(() => {
        showFallback.value = true;
        isOpening.value = false;
    }, 2500);
}

const stars = ref<{ x: number; y: number; size: number; opacity: number; delay: number }[]>([]);

onMounted(() => {
    for (let i = 0; i < 80; i++) {
        stars.value.push({
            x: Math.random() * 100,
            y: Math.random() * 100,
            size: Math.random() * 2 + 0.5,
            opacity: Math.random() * 0.6 + 0.2,
            delay: Math.random() * 3,
        });
    }

    openInApp();
});
</script>

<template>
    <Head :title="`${share.pilotCount} Pilots Shared`" />

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
        <div class="absolute inset-0 bg-gradient-to-r from-eve-orange/5 via-transparent to-eve-cyan/5" />

        <!-- Scan line effect -->
        <div class="pointer-events-none absolute inset-0 overflow-hidden">
            <div
                class="absolute inset-0 bg-[repeating-linear-gradient(0deg,transparent,transparent_2px,rgba(0,212,255,0.03)_2px,rgba(0,212,255,0.03)_4px)]"
            />
        </div>

        <!-- Main content -->
        <div class="relative z-10 flex min-h-screen flex-col items-center justify-center px-6 py-20">
            <!-- Header -->
            <div class="mb-8 text-center">
                <a href="/" class="inline-flex items-center gap-3 transition-opacity hover:opacity-80">
                    <svg class="h-10 w-10 drop-shadow-[0_0_10px_rgba(0,212,255,0.4)]" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                        <rect x="0" y="0" width="24" height="24" rx="4.5" fill="#12151a" />
                        <g stroke="#00d4ff" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none">
                            <circle cx="12" cy="12" r="7" />
                            <line x1="19" x2="16" y1="12" y2="12" />
                            <line x1="8" x2="5" y1="12" y2="12" />
                            <line x1="12" x2="12" y1="8" y2="5" />
                            <line x1="12" x2="12" y1="19" y2="16" />
                        </g>
                    </svg>
                    <h1 class="text-2xl font-bold tracking-wider text-eve-text-1">TELESCOPE</h1>
                </a>
            </div>

            <!-- Share card -->
            <div class="w-full max-w-lg">
                <div class="relative overflow-hidden rounded-lg border border-eve-border bg-eve-bg-1/90 backdrop-blur-sm">
                    <!-- Header -->
                    <div class="border-b border-eve-border bg-eve-bg-2/50 px-6 py-4">
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-3">
                                <div class="flex h-10 w-10 items-center justify-center rounded-lg bg-eve-orange/20">
                                    <svg class="h-5 w-5 text-eve-orange" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                                        <path
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                            d="M7.217 10.907a2.25 2.25 0 100 2.186m0-2.186c.18.324.283.696.283 1.093s-.103.77-.283 1.093m0-2.186l9.566-5.314m-9.566 7.5l9.566 5.314m0 0a2.25 2.25 0 103.935 2.186 2.25 2.25 0 00-3.935-2.186zm0-12.814a2.25 2.25 0 103.933-2.185 2.25 2.25 0 00-3.933 2.185z"
                                        />
                                    </svg>
                                </div>
                                <div>
                                    <h2 class="text-sm font-semibold tracking-wider text-eve-text-1">SHARED INTEL SCAN</h2>
                                    <p class="text-xs text-eve-text-3">{{ share.pilotCount }} pilots in this scan</p>
                                </div>
                            </div>
                            <div class="text-right text-xs text-eve-text-3">
                                <div class="flex items-center gap-1">
                                    <svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                                        <path
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                            d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z"
                                        />
                                        <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                                    </svg>
                                    {{ share.views }}
                                </div>
                            </div>
                        </div>
                    </div>

                    <!-- Pilot list preview -->
                    <div class="max-h-64 overflow-y-auto border-b border-eve-border p-4">
                        <div class="flex flex-wrap gap-2">
                            <span
                                v-for="(name, i) in share.pilots.slice(0, 50)"
                                :key="i"
                                class="inline-flex items-center rounded border border-eve-border bg-eve-bg-2 px-2 py-1 text-xs text-eve-text-2"
                            >
                                {{ name }}
                            </span>
                            <span
                                v-if="share.pilots.length > 50"
                                class="inline-flex items-center rounded border border-eve-border/50 bg-eve-bg-2/50 px-2 py-1 text-xs text-eve-text-3"
                            >
                                +{{ share.pilots.length - 50 }} more
                            </span>
                        </div>
                    </div>

                    <!-- Actions -->
                    <div class="p-6">
                        <button
                            v-if="!showFallback"
                            @click="openInApp"
                            :disabled="isOpening"
                            class="group relative w-full overflow-hidden rounded-lg border border-eve-cyan bg-eve-cyan/10 px-6 py-4 transition-all hover:bg-eve-cyan/20 hover:shadow-[0_0_30px_rgba(0,212,255,0.2)] disabled:opacity-50"
                        >
                            <div
                                class="absolute inset-0 bg-gradient-to-r from-eve-cyan/0 via-eve-cyan/10 to-eve-cyan/0 opacity-0 transition-opacity group-hover:opacity-100"
                            />
                            <div class="relative flex items-center justify-center gap-3">
                                <svg v-if="isOpening" class="h-5 w-5 animate-spin text-eve-cyan" fill="none" viewBox="0 0 24 24">
                                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                    <path
                                        class="opacity-75"
                                        fill="currentColor"
                                        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                                    ></path>
                                </svg>
                                <svg v-else class="h-5 w-5 text-eve-cyan" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25"
                                    />
                                </svg>
                                <span class="font-semibold tracking-wider text-eve-cyan">
                                    {{ isOpening ? 'OPENING TELESCOPE...' : 'OPEN IN TELESCOPE' }}
                                </span>
                            </div>
                        </button>

                        <!-- Fallback message -->
                        <div v-else class="space-y-4">
                            <div class="rounded-lg border border-eve-yellow/30 bg-eve-yellow/10 p-4">
                                <div class="flex gap-3">
                                    <svg
                                        class="h-5 w-5 shrink-0 text-eve-yellow"
                                        fill="none"
                                        viewBox="0 0 24 24"
                                        stroke="currentColor"
                                        stroke-width="1.5"
                                    >
                                        <path
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                            d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z"
                                        />
                                    </svg>
                                    <div>
                                        <p class="text-sm font-medium text-eve-yellow">Telescope not installed?</p>
                                        <p class="mt-1 text-xs text-eve-text-2">Download Telescope to view this scan with full threat analysis.</p>
                                    </div>
                                </div>
                            </div>

                            <div class="flex gap-3">
                                <a
                                    href="/"
                                    class="flex-1 rounded-lg border border-eve-border bg-eve-bg-2 px-4 py-3 text-center text-sm font-medium tracking-wider text-eve-text-2 transition-all hover:border-eve-cyan hover:text-eve-cyan"
                                >
                                    DOWNLOAD APP
                                </a>
                                <button
                                    @click="openInApp"
                                    class="flex-1 rounded-lg border border-eve-cyan/50 bg-eve-cyan/10 px-4 py-3 text-center text-sm font-medium tracking-wider text-eve-cyan transition-all hover:bg-eve-cyan/20"
                                >
                                    TRY AGAIN
                                </button>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Copy pilots fallback -->
                <div class="mt-4 flex items-center justify-center gap-4">
                    <button
                        @click="copyPilots"
                        class="inline-flex items-center gap-2 rounded border border-eve-border bg-eve-bg-2 px-4 py-2 text-xs tracking-wider text-eve-text-2 transition-all hover:border-eve-cyan hover:text-eve-cyan"
                    >
                        <svg
                            v-if="pilotsCopied"
                            class="h-3.5 w-3.5 text-green-400"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                        </svg>
                        <svg v-else class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                d="M15.666 3.888A2.25 2.25 0 0013.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 01-.75.75H9a.75.75 0 01-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 01-2.25 2.25H6.75A2.25 2.25 0 014.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 011.927-.184"
                            />
                        </svg>
                        {{ pilotsCopied ? 'COPIED!' : 'COPY PILOT NAMES' }}
                    </button>
                    <span class="text-xs text-eve-text-3">
                        Code: <code class="rounded bg-eve-bg-2 px-2 py-0.5 font-mono text-eve-text-2">{{ share.code }}</code>
                    </span>
                </div>
            </div>
        </div>

        <!-- Bottom glow -->
        <div class="absolute bottom-0 left-1/2 h-px w-1/2 -translate-x-1/2 bg-gradient-to-r from-transparent via-eve-orange/50 to-transparent" />
    </div>
</template>
