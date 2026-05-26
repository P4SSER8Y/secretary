<script setup lang="ts">
import { useDeviceStore } from '../lib/store';

const store = useDeviceStore();
</script>

<template>
    <div class="flex items-center gap-3 px-4 py-1.5 bg-base-200 text-xs border-b border-base-300 max-md:gap-2">
        <span class="flex items-center gap-1">
            <span
                class="w-2 h-2 rounded-full"
                :class="store.deviceState.online ? 'bg-success' : 'bg-error'"
            ></span>
            {{ store.deviceState.online ? '在线' : '离线' }}
        </span>
        <span class="text-base-content/60">
            IP: {{ store.deviceState.ip || '—' }}
        </span>
        <span v-if="store.deviceState.status" class="badge badge-xs">
            {{ store.deviceState.status }}
        </span>
        <span v-if="store.deviceState.last_seen" class="text-base-content/40 max-md:hidden">
            {{ store.deviceState.last_seen }}
        </span>
    </div>
</template>
