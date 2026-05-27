<script setup lang="ts">
import { computed } from 'vue';
import type { ImageMeta } from '../lib/structs';

const props = defineProps<{ imageId: string; images: ImageMeta[] }>();
const emit = defineEmits<{ close: []; switch: [id: string]; delete: [id: string] }>();

const meta = computed(() => props.images.find((m) => m.id === props.imageId));
const index = computed(() => props.images.findIndex((m) => m.id === props.imageId));

function prevImage() {
    if (props.images.length < 2) return;
    const i = index.value;
    const prev = i > 0 ? props.images[i - 1] : props.images[props.images.length - 1];
    emit('switch', prev.id);
}
function nextImage() {
    if (props.images.length < 2) return;
    const i = index.value;
    const next = i < props.images.length - 1 ? props.images[i + 1] : props.images[0];
    emit('switch', next.id);
}
</script>

<template>
    <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/70" @click.self="emit('close')">
        <div class="bg-base-100 rounded-lg shadow-2xl max-w-2xl w-full mx-4 max-h-[90dvh] overflow-auto flex flex-col">
            <!-- toolbar -->
            <div class="flex items-center gap-2 p-3 border-b border-base-300">
                <span class="text-sm font-medium truncate flex-1">{{ meta?.original_filename }}</span>
                <button class="btn btn-xs btn-ghost" @click="prevImage">◀</button>
                <span class="text-xs text-base-content/50">{{ index + 1 }}/{{ images.length }}</span>
                <button class="btn btn-xs btn-ghost" @click="nextImage">▶</button>
                <button class="btn btn-xs btn-outline btn-success" @click="emit('switch', imageId)">Send</button>
                <button class="btn btn-xs btn-outline btn-error" @click="emit('delete', imageId)">Delete</button>
                <button class="btn btn-xs btn-ghost" @click="emit('close')">✕</button>
            </div>
            <!-- image -->
            <div class="flex-1 flex items-center justify-center p-4 bg-base-300/30">
                <img
                    :src="`/album/api/preview/${imageId}`"
                    class="max-w-full max-h-[70dvh] object-contain rounded"
                    style="image-rendering: pixelated;"
                />
            </div>
            <!-- meta -->
            <div v-if="meta" class="p-3 border-t border-base-300 text-xs text-base-content/60 grid grid-cols-2 gap-1">
                <div>ID: <span class="font-mono">{{ meta.id.slice(0, 8) }}…</span></div>
                <div>Created: {{ meta.created_at.slice(0, 19) }}</div>
                <div>Dither: {{ meta.params.dither ? 'on' : 'off' }}</div>
                <div>Crop: {{ meta.params.crop_mode || 'fit' }}</div>
                <div v-if="meta.params.rotate_cw">Rotate: CW</div>
                <div v-if="meta.params.rotate_ccw">Rotate: CCW</div>
            </div>
        </div>
    </div>
</template>
