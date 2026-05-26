<script setup lang="ts">
import type { ImageMeta } from '../lib/structs';
import GalleryCard from './GalleryCard.vue';

defineProps<{ images: ImageMeta[] }>();
const emit = defineEmits<{ preview: [id: string]; switch: [id: string]; delete: [id: string] }>();
</script>

<template>
    <div v-if="images.length === 0" class="flex items-center justify-center py-20 text-base-content/40">
        <p>No images yet. Upload one!</p>
    </div>
    <div v-else class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3 p-4">
        <GalleryCard
            v-for="img in images"
            :key="img.id"
            :image="img"
            @preview="emit('preview', img.id)"
            @switch="emit('switch', img.id)"
            @delete="emit('delete', img.id)"
        />
    </div>
</template>
