<script setup lang="ts">
import { Head } from '@inertiajs/vue3';
import { ref } from 'vue';

const props = defineProps<{
    token: string;
    deepLink: string;
    characterName: string;
}>();

const copied = ref(false);

function copyToken() {
    navigator.clipboard.writeText(props.token);
    copied.value = true;
    setTimeout(() => (copied.value = false), 2000);
}

function openInApp() {
    window.location.href = props.deepLink;
}
</script>

<template>
    <Head title="Authentication" />

    <div class="flex min-h-screen items-center justify-center bg-eve-bg-0 p-4">
        <div class="w-full max-w-md space-y-6 text-center">
            <div class="mb-2 text-4xl text-eve-cyan">&#x2713;</div>

            <h1 class="text-xl font-bold tracking-wider text-eve-text-1">Authenticated as {{ characterName }}</h1>

            <p class="text-sm text-eve-text-2">Click below to open Telescope, or copy the token to paste it manually.</p>

            <button
                class="w-full cursor-pointer rounded bg-eve-cyan py-3 text-sm font-bold tracking-wider text-eve-bg-0 transition-all hover:bg-eve-cyan-dim"
                @click="openInApp"
            >
                OPEN IN TELESCOPE
            </button>

            <div class="space-y-2">
                <p class="text-[10px] tracking-widest text-eve-text-3">OR COPY TOKEN</p>
                <div
                    class="flex cursor-pointer items-center gap-2 rounded border border-eve-border bg-eve-bg-2 px-3 py-2 transition-colors hover:border-eve-cyan"
                    @click="copyToken"
                >
                    <code class="flex-1 truncate text-left text-xs text-eve-text-2">{{ token }}</code>
                    <span class="shrink-0 text-[10px] text-eve-cyan">
                        {{ copied ? 'COPIED!' : 'COPY' }}
                    </span>
                </div>
            </div>
        </div>
    </div>
</template>
