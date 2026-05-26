<script setup lang="ts">
import type { ImageMeta } from '../lib/structs';

defineProps<{ image: ImageMeta }>();
const emit = defineEmits<{ preview: []; switch: []; delete: [] }>();

const baseUrl = '/album/api';
</script>

<template>
    <div
        class="card bg-base-200 shadow-sm hover:shadow-md transition-shadow cursor-pointer group"
        @click="emit('preview')"
    >
        <figure class="aspect-[5/3] bg-base-300 overflow-hidden">
            <img
                :src="`${baseUrl}/preview/${image.id}`"
                :alt="image.original_filename"
                class="w-full h-full object-cover group-hover:scale-105 transition-transform"
                loading="lazy"
            />
        </figure>
        <div class="card-body p-2 text-xs">
            <div class="truncate font-medium">{{ image.original_filename }}</div>
            <div class="text-base-content/50">{{ image.created_at.slice(0, 10) }}</div>
            <div class="flex gap-1 mt-1">
                <button
                    class="btn btn-xs btn-outline btn-success flex-1"
                    @click.stop="emit('switch')"
                    title="发送到设备"
                >Send</button>
                <button
                    class="btn btn-xs btn-outline btn-error"
                    @click.stop="emit('delete')"
                    title="删除"
                >✕</button>
            </div>
        </div>
    </div>
</template>
